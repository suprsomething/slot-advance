# slotadvance.com

The slot.advance website: one static page, served by Cloudflare (Workers with static assets).

| File | What it is |
|---|---|
| `public/index.html` | The page. |
| `public/logo.png` | The striped slot.advance logo, cut from `artwork/bootlogo/slot-advance-logo-source.png` on a transparent background. |
| `public/og.jpg` | Link-preview image, cropped from the README hero. |
| `public/favicon.svg` | Browser-tab icon: the six logo stripes. |
| `public/slot-advance-0.4.0.mp4` | The 0.4.0 feature video, played on the page (H.264 1080p with AAC audio, about 14.5 MB; Cloudflare's limit is 25 MiB per file). |
| `public/video-poster.jpg` | The video's end card, shown before it plays. |
| `wrangler.jsonc` | Cloudflare settings: serve everything in `public/`. |

Cloudflare builds from this folder (root directory `website`). A change pushed to `main` goes live automatically once the repository is connected.
