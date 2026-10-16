// Throwaway spike for Phase 2. Usage:
//   dump <client.realm> [schemaVersion|-]   read-only dynamic open, print counts and samples
//   watch <client.realm> <seconds>          poll LastWriteTime, re-read counts on change
//   write <copy of client.realm>            add a dummy BeatmapCollection (COPY ONLY)
using Realms;

var cmd = args.Length > 0 ? args[0] : "";
var path = args.Length > 1 ? args[1] : "";
if (cmd == "write" && path.Contains(@"\Roaming\osu", StringComparison.OrdinalIgnoreCase))
    throw new InvalidOperationException("refusing to write under the real lazer folder");

RealmConfiguration Cfg(bool ro, ulong? schema) => new(path)
{
    IsReadOnly = ro,
    IsDynamic = true, // schema is taken from the file, no typed models needed
    SchemaVersion = schema ?? 0,
};

switch (cmd)
{
    case "dump": Dump(path, args.Length > 2 && args[2] != "-" ? ulong.Parse(args[2]) : null); break;
    case "watch": Watch(path, int.Parse(args[2])); break;
    case "write": Write(); break;
    case "head": Console.WriteLine(Head(path) + " mtime=" + File.GetLastWriteTimeUtc(path).ToString("O") + " lockMtime=" + File.GetLastWriteTimeUtc(path + ".lock").ToString("O")); break;
    default: Console.WriteLine("dump | watch | write"); break;
}

string Counts(Realm r)
{
    var sets = r.DynamicApi.All("BeatmapSet");
    var maps = r.DynamicApi.All("Beatmap");
    var cols = r.DynamicApi.All("BeatmapCollection");
    return $"sets={sets.Count()} beatmaps={maps.Count()} collections={cols.Count()}";
}

void Dump(string p, ulong? schema)
{
    using var r = Realm.GetInstance(Cfg(true, schema));
    Console.WriteLine($"opened read-only, file schema_version={r.Config.SchemaVersion}, {Counts(r)}");
    Console.WriteLine("schema classes: " + string.Join(",", r.Schema.Select(s => s.Name)));
    int hidden = 0, del = 0, shown = 0, noAudio = 0;
    var audio = new HashSet<string>();
    var samples = new List<string>();
    foreach (var set in r.DynamicApi.All("BeatmapSet"))
    {
        var dp = set.DynamicApi.Get<bool>("DeletePending");
        var files = set.DynamicApi.GetList<IEmbeddedObject>("Files")
            .ToDictionary(f => f.DynamicApi.Get<string>("Filename"),
                          f => f.DynamicApi.Get<IRealmObjectBase>("File").DynamicApi.Get<string>("Hash"));
        var added = set.DynamicApi.Get<DateTimeOffset>("DateAdded");
        foreach (var b in set.DynamicApi.GetList<IRealmObjectBase>("Beatmaps"))
        {
            var h = b.DynamicApi.Get<bool>("Hidden");
            if (dp) del++; if (h) hidden++;
            var m = b.DynamicApi.Get<IRealmObjectBase>("Metadata");
            var af = m.DynamicApi.Get<string>("AudioFile");
            if (!files.TryGetValue(af, out var hash)) { noAudio++; continue; }
            if (dp || h) continue;
            shown++;
            if (audio.Add(hash) && samples.Count < 3)
                samples.Add($"{m.DynamicApi.Get<string>("Title")} | {m.DynamicApi.Get<string>("TitleUnicode")} | {m.DynamicApi.Get<string>("Artist")} | {m.DynamicApi.Get<string>("ArtistUnicode")} | src={m.DynamicApi.Get<string>("Source")} | tags={Trunc(m.DynamicApi.Get<string>("Tags"))} | author={m.DynamicApi.Get<IRealmObjectBase>("Author").DynamicApi.Get<string>("Username")} | bpm={b.DynamicApi.Get<double>("BPM"):F0} len={b.DynamicApi.Get<double>("Length") / 1000:F0}s | added={added:yyyy-MM-dd} | audio={hash[..8]}.. hidden={h} delpend={dp}");
        }
    }
    Console.WriteLine($"beatmaps: deletePending={del} hidden={hidden} audioMissingInFiles={noAudio} visible={shown} distinctAudioHash={audio.Count}");
    foreach (var s in samples) Console.WriteLine("  " + s);
    foreach (var c in r.DynamicApi.All("BeatmapCollection").AsEnumerable().Take(3))
        Console.WriteLine($"  collection '{c.DynamicApi.Get<string>("Name")}' hashes={c.DynamicApi.GetList<string>("BeatmapMD5Hashes").Count}");
}

static string Trunc(string s) => s.Length > 30 ? s[..30] + "…" : s;

void Watch(string p, int seconds)
{
    // Keep ONE read-only realm open to test whether Refresh() sees external writes.
    using var r = Realm.GetInstance(Cfg(true, null));
    var last = File.GetLastWriteTimeUtc(p);
    int notified = 0;
    try { r.DynamicApi.All("BeatmapCollection").SubscribeForNotifications((_, _) => notified++); }
    catch (Exception e) { Console.WriteLine("subscribe failed: " + e.GetType().Name + ": " + e.Message); }
    Console.WriteLine($"watching: {Counts(r)}");
    var end = DateTime.UtcNow.AddSeconds(seconds);
    while (DateTime.UtcNow < end)
    {
        Thread.Sleep(500);
        var now = File.GetLastWriteTimeUtc(p);
        if (now == last) continue;
        last = now;
        string rf;
        try { rf = r.Refresh().ToString(); } catch (Exception e) { rf = "throws " + e.GetType().Name; }
        Console.WriteLine($"mtime changed {now:O}; old instance: {Counts(r)}; Refresh()={rf}; notifications={notified}");
        using var fresh = Realm.GetInstance(Cfg(true, null));
        Console.WriteLine($"  fresh read-only reopen: {Counts(fresh)}");
    }
    Console.WriteLine($"done, notification callbacks={notified}");
}

void Write()
{
    using var r = Realm.GetInstance(Cfg(false, 52));
    r.Write(() =>
    {
        var c = r.DynamicApi.CreateObject("BeatmapCollection", Guid.NewGuid());
        c.DynamicApi.Set("Name", "spike-dummy");
    });
    Console.WriteLine($"wrote dummy collection: {Counts(r)}");
}

// Header probe: Realm flips top_ref slots and the selector bit on every commit, so these 24 bytes change per commit.
// (cmd "head" is handled below because top-level statements run first; see switch.)
static string Head(string p)
{
    using var fs = new FileStream(p, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
    var b = new byte[24];
    fs.ReadExactly(b);
    return Convert.ToHexString(b);
}
