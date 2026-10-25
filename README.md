<p align="center"><img src="app/public/icon.svg" width="96" alt=""></p>

<h1 align="center">Retroflex</h1>

<p align="center"><b>Listen together, without a center.</b></p>

Retroflex is a small listening-room system for people who own their music. Two
or more people hear the same song at the same moment, as if sharing one pair of
earphones in the back of a classroom, while typing to each other in a room that
feels like an early-internet chat. If you want, they can watch each other's
words appear letter by letter, hesitations and backspaces included.

There are no accounts, no database and no recommendation feed. Music stays on
the machine that owns it. A room exists only in a hub's memory while someone is
in it, and a few minutes after the last person leaves it is gone.

- **Web**, installable as a PWA (this is also the iPhone and iPad path for now)
- **macOS, Windows, Linux** via Tauri v2
- **Android** via Tauri v2 (APK)
- **iOS** via Tauri v2 (build in Xcode with your own signing team)

Every platform runs the same interface, styled after Apple Music and Lyricify:
a fluid backdrop painted from the cover, a large cover with transport controls,
and lyrics that scroll and blur around the current line. The interface is made
of icons and content only, with no explanatory text.

## How it fits together

```mermaid
flowchart LR
  subgraph home["Your machine (or NAS)"]
    lib[("Music folder<br/>or AList")] --> st["retroflex station"]
  end
  subgraph vps["Any small VPS"]
    hub["retroflex hub<br/>rooms · clock · relay"]
  end
  st -- "one outbound WebSocket<br/>(multiplexed, credit flow control)" --> hub
  hub <-- "WebSocket + HTTP range" --> a["Listener A<br/>web / desktop / mobile"]
  hub <-- "WebSocket + HTTP range" --> b["Listener B"]
  a -. "chat sealed with a key<br/>that never leaves the link" .- b
```

**Station.** Indexes a local folder or an [AList](https://alist.nn.ci) share,
reads tags, covers and lyrics (embedded, or `.lrc` and `cover.jpg` sidecars),
makes thumbnails, and keeps a probe memo so rescans are cheap. It never opens a
port: it dials the hub and answers requests over that one connection.

**Hub.** Holds rooms in memory, answers clock pings, relays the station's bytes
to listeners with HTTP range support, and serves the web app embedded in its own
binary. It keeps nothing on disk. Anyone can run one; a link always names the hub
it belongs to.

**App.** One Svelte 5 code base. The browser gets it from the hub; desktop and
mobile builds wrap the same files in Tauri v2.

Both server roles live in a single static Rust binary, `retroflex`.

## Staying in step

Each client estimates the hub's clock from a burst of pings and then keeps
pinging. It trusts the reply with the shortest round trip, because the
shortest exchange has the least room for asymmetric delay (Cristian's
algorithm). Small corrections are eased in; large ones are taken at once.

The hub never says "play now". It says "this track, from this second, starting
at hub time T", and puts T about 450 ms in the future so every listener can
seek and buffer before it arrives. After that each client compares what its
audio element is playing with where the room's timeline says it should be:

| drift | response |
| --- | --- |
| under 15 ms | nothing |
| 15 to 350 ms | nudge `playbackRate` by up to 4% until it closes, pitch preserved |
| over 350 ms | seek |

Where the browser reports it, output latency (for example Bluetooth earbuds) is
added so the moment you *hear* matches, not the moment the decoder runs. Over a
260 ms round trip between continents, test clients held within about ±12 ms of
the shared timeline.

## What the hub can and cannot see

| | hub sees | hub does not see |
| --- | --- | --- |
| audio | bytes it relays, track titles in the queue | |
| people | how many connections a room has | names, colours |
| words | ciphertext lengths, so how many characters someone typed | chat lines, live drafts |

A room link looks like `https://hub/#/r/<room>/<secret>`. The part after `#` is
never sent to any server. From the secret, each client derives two independent
keys with HKDF-SHA256: a **ticket** that the hub checks before letting you in or
streaming you audio, and a **seal** key that encrypts chat, drafts and profiles
with XChaCha20-Poly1305. The hub learns the ticket but cannot derive the seal
from it.

To open a room you need the **station key**, printed by `retroflex station` as a
station link. Hand that only to people you would hand your record collection.

Chat text is sealed as UTF-32, four bytes per character, so the hub can enforce
a budget of 450 characters per person per minute from the ciphertext length
alone. A client that skips the check in its own code still gets turned away.

Two conveniences trade a little of this for ease, and say so:

- **Invite codes.** A six-letter code (for example `K7Q M2P`) stands in for a
  room link for ten minutes. To make that work the hub holds the room secret
  for those ten minutes. The QR code and the plain link keep the secret off the
  hub entirely.
- **Lyric translation.** When a song's lyrics have no translation of their own,
  the hub sends the sung lines to Google Translate in the listener's language
  and caches the result for a week. Songs already in that language are skipped.

## Public rooms

A station started with `--open` can be browsed without a key, and its
listeners can drop into a public room from the globe button in the library.
The hub picks the open public room that fits best by size (rooms of about four
feel liveliest), whether music is playing, and how many artists overlap with
what you have been listening to. If none fits, a new public room opens.

Public rooms keep the music going and keep it fair:

| | rule |
| --- | --- |
| seats | 12 |
| pause, seek, rewind, jump | not available; the centre button mutes your own device |
| skip | when more than half of the room votes for it |
| adding | to the end only, two songs per person waiting, ten waiting in total |
| removing | your own songs and automatic picks, never the one playing |

Private rooms work as before: anyone inside can do anything.

## Quick start

You need Rust (stable) and Node 22.

```sh
cd app && npm ci && npm run build && cd ..
cargo build --release
```

Run a hub (here on your own machine):

```sh
RETROFLEX_HUB_TOKEN=change-me ./target/release/retroflex hub --listen 127.0.0.1:7746
```

Share a library through it:

```sh
RETROFLEX_HUB_TOKEN=change-me ./target/release/retroflex station \
  --name home --hub http://127.0.0.1:7746 \
  -l ~/Music \
  -l "http://nas.local:5244/Music"
```

The station prints a station link. Open it, choose something, and a room is
created; the link icon copies the room link for whoever you want beside you.
Start the station with `--open` and the hub's front page becomes the library
itself: anyone who visits can pick a song and get a room of their own.

When a queue runs dry and nobody has added anything, the hub draws three random
tracks from the library and keeps playing. They are marked in the queue and can
be removed like any other track; removing them does not summon more.

| flag / env | used by | meaning |
| --- | --- | --- |
| `--listen` / `RETROFLEX_LISTEN` | hub | address to bind, default `127.0.0.1:7746` |
| `--token` / `RETROFLEX_HUB_TOKEN` | both | shared secret that admits stations |
| `--name` / `RETROFLEX_NAME` | station | name used in links, `a-z 0-9 -` |
| `--library`, `-l` / `RETROFLEX_LIBRARY` | station | folder, AList folder URL or `r2://bucket/prefix`; repeatable, `;`-separated in the env |
| `--hub` / `RETROFLEX_HUB` | station | hub URL, `https://` or `http://` |
| `--alist-token` / `RETROFLEX_ALIST_TOKEN` | station | for private AList folders |
| `--cache` / `RETROFLEX_CACHE` | station | tags, covers, thumbnails and the station key |
| `--rescan` | station | minutes between rescans, default 30 |
| `--open` / `RETROFLEX_OPEN` | station | let any visitor of the hub browse and open rooms, no key needed |
| `--public` / `RETROFLEX_PUBLIC_URL` | station | hub URL to print in links when `--hub` points at localhost |
| `--s3-endpoint`, `--s3-key`, `--s3-secret` / `RETROFLEX_S3_*` | station, sync | bucket credentials |
| `--relay` / `RETROFLEX_RELAY` | station | never hand out raw bucket-endpoint links |
| `--media-url`, `--media-secret` / `RETROFLEX_MEDIA_*` | station | media Worker that serves the bucket on your own domain |
| `--hot-mb` / `RETROFLEX_HOT_MB` | station | disk for recently relayed bucket audio, default 2048 |

Formats are played by the platform's own decoder, so anything the browser or
webview can play works: MP3, AAC/M4A, FLAC, Ogg Vorbis/Opus, WAV, WebM. Lyrics
can be LRC, including translation lines that share a timestamp, or plain text.
Legacy GBK lyric files are decoded too.

## Hosting a hub on a VPS

The hub needs about 10 MB of RAM and no disk. `deploy/install.sh` puts it on a
Linux machine next to whatever else is there: everything goes under
`/opt/retroflex` and `/etc/retroflex`, both services run with
`DynamicUser=yes`, and nothing listens on a public port, because traffic arrives
through a Cloudflare Tunnel.

```sh
cloudflared tunnel create retroflex
cloudflared tunnel route dns retroflex listen.example.org
cargo zigbuild --release --target x86_64-unknown-linux-musl
scp target/x86_64-unknown-linux-musl/release/retroflex ~/.cloudflared/<id>.json deploy/* root@vps:/tmp/rf/
ssh root@vps 'cd /tmp/rf && sh install.sh ./retroflex ./<id>.json listen.example.org'
```

The installer prints the hub token for your stations.

### Music in a bucket

A small VPS has little disk and every byte it relays costs it bandwidth. Put
the library in an S3-compatible bucket instead, Cloudflare R2 being the obvious
fit (no egress fees). The station then reads tags and covers from the bucket,
and when someone presses play it answers with a short-lived signed link, so the
audio flows from the bucket's edge straight to the listener and never crosses
the VPS or the tunnel.

1. Create a private bucket and give it this CORS policy:
   `[{"AllowedOrigins":["*"],"AllowedMethods":["GET","HEAD"],"AllowedHeaders":["Range"],"ExposeHeaders":["Content-Length","Content-Range","Accept-Ranges"],"MaxAgeSeconds":86400}]`
2. Create an API token with **Object Read & Write** on that bucket only, without
   an IP filter (listeners use links signed with it).
3. Store the credentials on the hub machine with `sh deploy/r2-setup.sh`, which
   writes `/etc/retroflex/r2.env`.
4. Copy music in from wherever it lives today, a folder or an AList share; files
   already present with the same size are skipped, nothing is ever deleted:

   ```sh
   retroflex sync --from ~/Music --to r2://retroflex/home
   ```
5. Point the station at the bucket: `RETROFLEX_LIBRARY=r2://retroflex/home`.

Rescans pick up new uploads every 30 minutes.

The raw bucket endpoint is not reachable everywhere. `worker/` holds a small
Cloudflare Worker that serves the bucket on your own subdomain instead: the
station signs each link with HMAC and a six-hour expiry, the Worker checks it
and streams from R2 with full Range and CORS support. Deploy it with
`cd worker && npx wrangler deploy`, set the same random string as its
`MEDIA_SECRET` (`npx wrangler secret put MEDIA_SECRET`) and as the station's
`RETROFLEX_MEDIA_SECRET`, and point `RETROFLEX_MEDIA_URL` at the subdomain.

If a listener still cannot reach it, the player retries through the station
(`?relay=1`). Relayed tracks are kept on the station's disk, up to
`RETROFLEX_HOT_MB` (2048 by default), dropping whatever was played longest ago.
`--relay` stops the station from ever handing out raw bucket-endpoint links.

### Music on the VPS disk

To keep a library on the VPS itself, copy it to `/srv/retroflex/music`, write
`RETROFLEX_NAME`, `RETROFLEX_HUB=http://127.0.0.1:7746` and
`RETROFLEX_PUBLIC_URL=https://listen.example.org` into
`/etc/retroflex/station.env` together with `RETROFLEX_LIBRARY=/srv/retroflex/music`,
and enable `deploy/retroflex-station.service`.

## Apps

Releases are built by GitHub Actions when a `v*` tag is pushed:

- `retroflex` binaries for Linux (x86_64, aarch64, static musl), macOS (Apple silicon) and Windows
- desktop bundles: universal `.dmg` for macOS, `.msi` and `.exe` for Windows, `.AppImage` and `.deb` for Linux
- an Android `.apk` signed with the `ANDROID_KEYSTORE` and `ANDROID_KEYSTORE_PASSWORD` secrets, so each release installs over the last (without them CI falls back to a throwaway key, and Android will refuse to update)

The apps can keep songs for offline listening: one tap on the download button
saves a song, an album or a saved list into `Music/Retroflex` on desktops or the
app's own storage on phones. A saved copy plays from disk but stays on the
room's clock. Saved lists live only in the apps and can be shared as links that
carry nothing but track ids.

iOS needs your own Apple team: `cd app && npx tauri ios init && npx tauri ios
build`. Until then, open any link in Safari and add it to the home screen; lock
screen controls work through the Media Session API.

In the desktop and mobile apps, paste a station or room link on the first
screen; the app remembers which hub it came from.

## Development

```sh
cargo run -- hub --token dev                    # terminal 1
cargo run -- station --name dev --hub http://127.0.0.1:7746 --token dev -l ~/Music   # terminal 2
cd app && npm run dev                           # http://localhost:5173, proxies /api to the hub
cd app && npm run tauri dev                     # desktop shell around the dev server
```

`cargo test` covers the protocol, ranges, scanning helpers and room logic;
`npm test` and `npm run check` cover the client.

```
crates/retroflex   hub + station (Rust: tokio, axum, lofty, ureq, image)
app/src/lib        room session, player, clock, crypto, lyrics, links
app/src/ui         Svelte 5 components
app/src-tauri      Tauri v2 shell
deploy             systemd units and installer
```

Every comment in the code base is a haiku, written in Japanese.

## Choices, and what was left out

This design started from a more maximal proposal. What survived, and why:

| proposed | here | reason |
| --- | --- | --- |
| Local-first, ephemeral rooms, no central database | kept | the point of the project |
| Rust core, Tauri v2, Svelte 5 runes | kept | one binary for servers, one UI for six platforms |
| Scheduled playback at a future timestamp | kept | the core of good sync |
| PLL-style resampling instead of hard seeks | kept, as `playbackRate` nudging | same idea, using the platform's own resampler |
| Live keystroke mode | kept | it is the best part |
| Leaderless shared queue | kept | everyone in a room has the same controls |
| Tiny blurred placeholders for covers | added | images resolve from a 1 KB blur instead of popping in |
| BLAKE3 | kept, for IDs and key hashes | streaming Merkle verification is unnecessary over TLS from your own station |
| Iroh / QUIC hole punching | left out | browsers cannot open raw UDP, so web clients would need a relay anyway; one outbound WebSocket per station is simpler and works behind any NAT |
| WebTransport / Media over QUIC | left out | uneven browser support, and plain HTTP range requests already stream well |
| MLS group encryption | left out | rooms have no membership management; a key carried in the link plus an AEAD covers the threat model |
| Loro CRDT for the queue | left out | the hub already orders every command, so there are no concurrent edits to merge |
| WASM decoder, AudioWorklet, SharedArrayBuffer | left out | needs cross-origin isolation, which breaks cross-origin media; native decoders are good enough |
| Kalman-filtered clock | left out | min-RTT selection is simpler and does as well here |
| Cloudflare Durable Objects | left out | a hub on a small VPS costs nothing extra and keeps the relay yours |

What the hub relays is still visible to whoever runs the hub. If that matters
to you, run your own; it is one command.

## License

[AGPL-3.0-or-later](LICENSE). If you run a modified hub for others, share the changes.
