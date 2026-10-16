// listenosu helper: reads osu!lazer's library and talks JSON Lines on stdin/stdout.
// The protocol is documented in docs/helper-protocol.md.
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using Realms;

const string HelperVersion = "0.1.0";
const int MaxAttempts = 3;
var settle = TimeSpan.FromMilliseconds(300);
var poll = TimeSpan.FromSeconds(1);

var stdout = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false)) { AutoFlush = true, NewLine = "\n" };
var gate = new object();
string? realmPath = null;
string? lastHead = null;

void Send(Action<Utf8JsonWriter> body)
{
    var buffer = new MemoryStream();
    using (var w = new Utf8JsonWriter(buffer))
    {
        w.WriteStartObject();
        body(w);
        w.WriteEndObject();
    }
    lock (gate)
        stdout.WriteLine(Encoding.UTF8.GetString(buffer.ToArray()));
}

void SendError(string code, string message) =>
    Send(w => { w.WriteString("type", "error"); w.WriteString("code", code); w.WriteString("message", message); });

var publishLock = new object();

// The watcher thread and the command loop both publish; one read at a time.
void Publish(string type)
{
    lock (publishLock)
        PublishLocked(type);
}

// Reads the whole library, retrying while lazer is writing; sends `type` on success.
void PublishLocked(string type)
{
    var path = realmPath!;
    for (var attempt = 1; attempt <= MaxAttempts; attempt++)
    {
        try
        {
            var before = Library.Head(path);
            var (songs, collections) = Library.Read(path);
            if (before != Library.Head(path))
            {
                if (attempt == MaxAttempts)
                    SendError("unstable_read", "client.realm kept changing while it was read");
                else
                    Thread.Sleep(settle);
                continue;
            }
            lastHead = before;
            Send(w => WriteLibrary(w, type, songs, collections));
            return;
        }
        catch (Exception e) when (attempt < MaxAttempts)
        {
            Console.Error.WriteLine($"read attempt {attempt} failed: {e.Message}");
            Thread.Sleep(settle);
        }
        catch (Exception e)
        {
            // Not a Realm-specific exception type on purpose: the message is what the user will paste.
            SendError(e is IOException ? "read_failed" : "open_failed", $"{e.GetType().Name}: {e.Message}");
            lastHead = TryHead(path);
            return;
        }
    }
}

string? TryHead(string path)
{
    try { return Library.Head(path); }
    catch (IOException) { return null; }
}

void WriteLibrary(Utf8JsonWriter w, string type, List<Song> songs, List<Collection> collections)
{
    w.WriteString("type", type);
    w.WriteStartArray("songs");
    foreach (var s in songs)
    {
        w.WriteStartObject();
        w.WriteString("audio_hash", s.AudioHash);
        w.WriteString("background_hash", s.BackgroundHash);
        w.WriteString("title", s.Title);
        w.WriteString("title_unicode", s.TitleUnicode);
        w.WriteString("artist", s.Artist);
        w.WriteString("artist_unicode", s.ArtistUnicode);
        w.WriteString("source", s.Source);
        w.WriteString("tags", s.Tags);
        w.WriteString("author", s.Author);
        w.WriteNumber("bpm", s.Bpm);
        w.WriteNumber("length_ms", s.LengthMs);
        w.WriteString("date_added", s.DateAdded);
        if (s.LastPlayed is { } played)
            w.WriteString("last_played", played);
        else
            w.WriteNull("last_played");
        WriteStrings(w, "beatmap_md5s", s.BeatmapMd5s);
        w.WriteEndObject();
    }
    w.WriteEndArray();
    w.WriteStartArray("collections");
    foreach (var c in collections)
    {
        w.WriteStartObject();
        w.WriteString("name", c.Name);
        WriteStrings(w, "beatmap_md5s", c.BeatmapMd5s);
        w.WriteEndObject();
    }
    w.WriteEndArray();
}

void WriteStrings(Utf8JsonWriter w, string name, List<string> values)
{
    w.WriteStartArray(name);
    foreach (var v in values)
        w.WriteStringValue(v);
    w.WriteEndArray();
}

// lazer's version is only known where its exe is in the usual per-user install location.
string? LazerVersion()
{
    if (!OperatingSystem.IsWindows())
        return null;
    var exe = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "osulazer", "current", "osu!.exe");
    return File.Exists(exe) ? FileVersionInfo.GetVersionInfo(exe).ProductVersion : null;
}

void Open(string dataDir)
{
    var path = Path.Combine(dataDir, "client.realm");
    if (!File.Exists(path))
    {
        SendError("no_database", $"client.realm was not found in {dataDir}");
        return;
    }
    realmPath = path;
    Send(w =>
    {
        w.WriteString("type", "version");
        w.WriteString("helper", HelperVersion);
        w.WriteString("realm", typeof(Realm).Assembly.GetName().Version?.ToString(3));
        w.WriteString("lazer", LazerVersion());
    });
    Publish("snapshot");
}

// Watches the header bytes; a change means lazer committed something.
var watcher = new Thread(() =>
{
    while (true)
    {
        Thread.Sleep(poll);
        var path = realmPath;
        if (path is null)
            continue;
        var head = TryHead(path);
        if (head is null || head == lastHead)
            continue;
        Thread.Sleep(settle);
        Publish("changed");
    }
})
{ IsBackground = true };
watcher.Start();

string? line;
while ((line = Console.In.ReadLine()) is not null)
{
    try
    {
        using var doc = JsonDocument.Parse(line);
        switch (doc.RootElement.GetProperty("cmd").GetString())
        {
            case "open": Open(doc.RootElement.GetProperty("data_dir").GetString()!); break;
            case "refresh" when realmPath is not null: Publish("changed"); break;
            case "quit": return;
            default: SendError("bad_command", line); break;
        }
    }
    catch (Exception e) when (e is JsonException or KeyNotFoundException or InvalidOperationException)
    {
        SendError("bad_command", e.Message);
    }
}
