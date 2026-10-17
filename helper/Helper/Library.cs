using Realms;

record Song(
    string AudioHash,
    string? BackgroundHash,
    int? OnlineId,
    string Title,
    string TitleUnicode,
    string Artist,
    string ArtistUnicode,
    string Source,
    string Tags,
    string Author,
    double Bpm,
    double LengthMs,
    DateTimeOffset DateAdded,
    DateTimeOffset? LastPlayed,
    List<string> BeatmapMd5s);

record Collection(string Name, List<string> BeatmapMd5s);

/// Reads lazer's client.realm. The file is opened read-only and dynamic: no typed models, so a
/// column lazer adds later does not break us, and we never write (docs/helper-protocol.md).
static class Library
{
    public static (List<Song> Songs, List<Collection> Collections) Read(string path)
    {
        var config = new RealmConfiguration(path) { IsReadOnly = true, IsDynamic = true };
        using var realm = Realm.GetInstance(config);

        // One song per audio hash, in library order; the first beatmap using it is the representative.
        var songs = new Dictionary<string, Song>();
        foreach (var set in realm.DynamicApi.All("BeatmapSet"))
        {
            if (set.DynamicApi.Get<bool>("DeletePending"))
                continue;
            var files = new Dictionary<string, string>();
            foreach (var usage in set.DynamicApi.GetList<IEmbeddedObject>("Files"))
            {
                var file = usage.DynamicApi.Get<IRealmObjectBase>("File");
                files[usage.DynamicApi.Get<string>("Filename")] = file.DynamicApi.Get<string>("Hash");
            }
            var added = set.DynamicApi.Get<DateTimeOffset>("DateAdded");
            // lazer stores -1 for a set that was not downloaded from osu!.
            var rawId = Optional<int>(set, "OnlineID");
            int? onlineId = rawId > 0 ? rawId : null;

            foreach (var beatmap in set.DynamicApi.GetList<IRealmObjectBase>("Beatmaps"))
            {
                if (beatmap.DynamicApi.Get<bool>("Hidden"))
                    continue;
                var meta = beatmap.DynamicApi.Get<IRealmObjectBase>("Metadata");
                if (!files.TryGetValue(meta.DynamicApi.Get<string>("AudioFile"), out var audio))
                    continue;

                var md5 = beatmap.DynamicApi.Get<string>("MD5Hash");
                var played = Optional<DateTimeOffset?>(beatmap, "LastPlayed");
                if (songs.TryGetValue(audio, out var existing))
                {
                    existing.BeatmapMd5s.Add(md5);
                    if (played > existing.LastPlayed)
                        songs[audio] = existing with { LastPlayed = played };
                    continue;
                }

                var background = meta.DynamicApi.Get<string>("BackgroundFile");
                songs[audio] = new Song(
                    audio,
                    files.TryGetValue(background, out var bg) ? bg : null,
                    onlineId,
                    meta.DynamicApi.Get<string>("Title"),
                    meta.DynamicApi.Get<string>("TitleUnicode"),
                    meta.DynamicApi.Get<string>("Artist"),
                    meta.DynamicApi.Get<string>("ArtistUnicode"),
                    meta.DynamicApi.Get<string>("Source"),
                    meta.DynamicApi.Get<string>("Tags"),
                    Optional<IRealmObjectBase>(meta, "Author")?.DynamicApi.Get<string>("Username") ?? "",
                    beatmap.DynamicApi.Get<double>("BPM"),
                    beatmap.DynamicApi.Get<double>("Length"),
                    added,
                    played,
                    [md5]);
            }
        }

        var collections = realm.DynamicApi.All("BeatmapCollection").AsEnumerable()
            .Select(c => new Collection(
                c.DynamicApi.Get<string>("Name"),
                c.DynamicApi.GetList<string>("BeatmapMD5Hashes").ToList()))
            .ToList();
        return (songs.Values.ToList(), collections);
    }

    /// Realm flips its top-ref slots on every commit, so these 24 bytes change per commit.
    public static string Head(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        var bytes = new byte[24];
        stream.ReadExactly(bytes);
        return Convert.ToHexString(bytes);
    }

    /// A column that older or newer lazer builds may lack reads as the default instead of failing.
    static T? Optional<T>(IRealmObjectBase obj, string name)
    {
        try { return obj.DynamicApi.Get<T>(name); }
        catch (Exception) { return default; }
    }
}
