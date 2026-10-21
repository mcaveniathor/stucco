# Optional Atelier font asset

The `atelier-fonts` feature registers `css/fonts.css` with Stucco's existing
asset registry. The CSS embeds two WOFF2 faces as data URLs, so there is no
third-party request, separate font route, or base-path handling. Add the
`showcase-fonts` class to an Atelier composition. All default builds use system
fonts, and named/guided theme exports do not silently enable this font feature.

The font programs are unmodified Google Fonts Latin web subsets downloaded on
21 October 2026. Both are SIL Open Font License 1.1; their copyright statements
and complete licenses are included here. Their source projects are
[DM Sans](https://github.com/googlefonts/dm-fonts) and
[Newsreader](https://github.com/productiontype/Newsreader).

| Face | Normal weights | WOFF2 bytes | Source |
| --- | --- | ---: | --- |
| DM Sans | 400–700 | 36,932 | [v17 Latin file](https://fonts.gstatic.com/s/dmsans/v17/rP2Yp2ywxg089UriI5-g4vlH9VoD8Cmcqbu0-K4.woff2) |
| Newsreader | 400–600 | 58,084 | [v26 Latin file](https://fonts.gstatic.com/s/newsreader/v26/cY9VfjOCX1hbuyalUrK49dLac06G1ZGsZBtoBAbNJYQ.woff2) |

SHA-256 before base64 encoding:

- DM Sans: `9fea608a947e67020c33cad9a6fe3d60c54119dfb8cff87768a8117a15ed7543`
- Newsreader: `62981321d9a3cc7a61a73792729043703fd6112da86e8ec848bb57f088578757`

Google's Latin coverage declaration for both files:
`U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC,
U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193,
U+2212, U+2215, U+FEFF, U+FFFD`.
This covers the showcase's French accented names. Other scripts, extended Latin
characters outside this set, and italic styling rely on platform fallback.
Do not describe this subset as full international coverage.

`font-display: optional` avoids a late font swap. DM Sans falls back to system
sans; Newsreader falls back to Iowan Old Style or Georgia. The default system
path is the reference for layout resilience. A restrictive CSP must allow
`font-src data:` to display this optional asset; otherwise fallback remains
usable. This feature adds roughly 127 KB of uncompressed CSS to a bundle, even
when the class is unused. Keep it off unless the pairing is wanted. There are no
font preloads and no remote dependencies at runtime.

Preview with `cargo run -p gallery --features atelier-fonts -- target/gallery-fonts`,
then serve that directory and open `showcase-fonts.html`.
