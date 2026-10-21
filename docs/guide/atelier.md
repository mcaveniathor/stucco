# The Atelier workspace

Atelier pairs a warm plaster canvas and terracotta accents with serif display
headings and system sans serif text. Fine rules separate groups; tables and
forms stay quiet. The [orders showcase](../gallery/showcase.html) renders the
same composition as the landing preview, in both schemes.

## Compose the theme

This is a composition of existing options, separate from the fourteen stable
presets and the deterministic seeded generator:

```rust
use stucco::theme::{ButtonDepth, Finish, Fonts, HeaderStyle, HeadingFont,
    HeadingWeight, Motion, NavStyle, PanelStyle, Preset, Radius, ShellLayout, Theme};

let theme = Theme::preset(Preset::Sand)
    .fonts(Fonts::system())
    .heading_font(HeadingFont::Serif)
    .heading_weight(HeadingWeight::Regular)
    .finish(Finish::Sand)
    .header_style(HeaderStyle::Plain)
    .shell_layout(ShellLayout::Topbar)
    .nav_style(NavStyle::Bar)
    .panel_style(PanelStyle::Ruled)
    .radius(Radius::Soft)
    .button_depth(ButtonDepth::Raised)
    .motion(Motion::Gentle);
```

`Heading::new(1, "Orders").display()` creates a responsive display title without
changing the body scale, form controls, table type or heading semantics. Use it
sparingly for the principal title. All text uses local font stacks; this recipe
has no font downloads. Heading fallbacks include Charter, Cambria, Georgia and
the platform serif. Bundled optional fonts are a later part of the design plan.

## Compose the workspace

The repository's `examples/showcase` crate contains copyable functions and
`css/showcase.css`. It is a workspace example, not a published dependency.
It owns presentation only; callers supply navigation, actions and data.

- Use `AppShell` with a small masthead and top navigation. Place the page title
  and primary action together in the main content, aligned with the table.
- Use `Avatar::decorative()` beside a written customer name. Put secondary
  context under the name. Never invent contact details for live records.
- Keep status text in `Badge`, and use tone to reinforce its meaning.
- Use `Stat` only for values the application can derive. The live example
  explicitly labels its metrics “on this page”; they are not global totals.
- Use `DataTable::responsive()` to render labelled cards at narrow container
  widths. Put the primary record link first, and keep amounts numeric so
  alignment and filtering still use the underlying value.
- Use `DataTable::compact_filters()` for a native “More filters & sorting”
  disclosure. Search and categorical filters stay visible. Ranges, sorting
  and page size remain in the same GET form, and active secondary choices
  reopen the disclosure after navigation. This works without scripts.

The actual orders example preserves CSRF, revision checks, native POST forms,
error summaries, confirmation pages and flash messages. Its gallery counterpart
is explicitly illustrative, with links to this recipe instead of dead actions.

## Review and adapt

Start with the shared stylesheet and adjust the page composition, not the
library defaults. Use `ThemeScope` for a bounded preview and `Surface` for local
emphasis. Keep long reading and data regions opaque. The theme engine checks
both schemes, and existing preference styles reduce decoration for increased
contrast, reduced motion and forced colors.

Run the capture workflow in `browser/README.md` to compare phone, tablet and
desktop layouts. Include long names, large amounts, empty results and failed
submissions when adapting the screen to your own data.

## More directions and screens

`Direction::Atelier.theme()` is the named form of this recipe. The other
signatures, guided generation, editorial and project compositions, and the
optional local font pairing are described in
[Signature directions and complete screens](visual-design.html).
