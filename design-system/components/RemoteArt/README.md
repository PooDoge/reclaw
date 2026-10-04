A picture from the internet in the place of a placeholder. Rust: `reclaw_ui::components::RemoteArt`, built on `reclaw_ui::media::use_remote_file`.

* The placeholder (`ArtPlaceholder`, which names the art's role: CAPSULE, HEADER, SCREENSHOT ...) shows until the picture is on disk, and stays if it never arrives: offline, no art, or "Download artwork and READMEs" switched off in Settings.
* The picture fills the box and is cropped to it, like cover art. SVGs are drawn with the toolkit's SVG viewer and fall back to the placeholder if they cannot be drawn.
* It never fetches by itself: the file comes from `reclaw-media`'s cache (https only, nothing on the local network, size and time capped, kind decided from the bytes). A component using it is keyed by the address.

A failed picture is **not** reported to the user per image; the placeholder is the whole message. A cache size and a Clear cache button are not built yet.
