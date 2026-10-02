# Agent brief: improvements to the Stucco design spec

Date: 2026-10-01
Target: `docs/superpowers/specs/2026-10-01-stucco-design.md`
Status: proposed changes for spec revision; not an implementation plan

## Task and intent

Revise the design spec to describe a composable server-rendered component library and a compatible, separately scoped Tower-based framework. Design their integration together and validate it through small end-to-end slices.

Preserve the current priorities: modularity, intuitive APIs, accessible native HTML, progressive enhancement, framework-independent rendering, and minimal shipped assets. Do not expand v1 to implement the entire catalog or framework at once.

This brief consolidates suggestions from the discussion. Treat the catalog and trait names as proposals, not approved public API. Produce a revised spec and identify decisions requiring user review before product implementation. Do not create a remote, publish, or add AI attribution to commits or PRs.

## 1. Clarify project boundaries (§1–3)

Keep the existing component-library spec focused on rendering, themes, assets, and components. Add an integration-contract section and create a separate framework spec when framework work is authorized.

- `stucco-core`: rendering, safe markup, stable identity support, asset requirements, pages and fragments.
- `stucco-theme`: token generation and theme validation.
- `stucco-ui`: primitives and compositions; accepts presentation state and endpoint descriptions without executing requests.
- `stucco-diagram`: optional diagram rendering and enhancement.
- Framework crate(s), naming to be decided: Tower services/layers, request extraction, response conversion, action execution, security integration, partial delivery, and optional live transports.
- `stucco`: retain its lightweight component facade. Decide separately whether the framework has its own facade or an optional feature; do not silently add server dependencies to existing defaults.

Compile the component library without Tower, an HTTP server, or an async runtime. Allow application-provided Tower services and domain sources to be adapted without adopting Stucco's entire framework.

## 2. Specify three levels of composition (§6)

1. **Primitives:** a single layout, semantic, visual, or interaction concern.
2. **Compositions:** recurring UI patterns with named slots and shared accessibility wiring.
3. **Shells and recipes:** page structure and documented examples assembled from public components.

All levels implement the same `Render` contract. Specify slots such as `heading`, `description`, `actions`, `body`, and `footer`, accepting arbitrary renderable children where appropriate.

Define landmark ownership: shells own the main landmark; nested panels do not emit additional main landmarks. Define heading semantics separately from visual size. Components own the IDs and ARIA relationships of their internal controls; supplied IDs must follow a documented uniqueness policy.

Expose useful constituents independently: `Feature` as well as `FeatureGrid`, `PricingTier` as well as `PricingTiers`, and disclosure items as well as their collections.

Compositions collect assets by rendering their children. Document feature dependencies and distinguish unavoidable dependencies from optional embellishments. Avoid requiring icons merely to render a text-only component.

## 3. Expand the catalog without making it all mandatory v1 (§6–7)

Use the following inventory to identify gaps. Retain existing spec components unless an explicit consolidation makes the API clearer. Annotate each entry with implementation phase, feature family, native fallback, optional behavior, and dependencies.

| Family | Proposed inventory |
|---|---|
| Layout/foundations | Stack, Cluster, Grid, Sidebar, Switcher, Center, Container, Cover, Frame, Reel, Spacer, Separator, VisuallyHidden, SkipLink, Icon, Image, Surface, ThemeScope |
| Typography/content | Heading, Text, Link, Code, CodeBlock, Kbd, Blockquote, Prose, List, ListItem, DescriptionList, Caption, Time |
| Actions | Button, IconButton, ButtonLink, ButtonGroup, ActionGroup, CopyButton, DownloadLink, ActionForm, AsyncAction, ConfirmAction |
| Form primitives | Form, Field, Fieldset, Legend, Input, Textarea, Select, Checkbox, CheckboxGroup, RadioGroup, Switch, Range, FileInput, PasswordInput, Combobox, HiddenInput, CsrfToken, FieldHint, FieldError, ErrorSummary |
| Composed forms | FormSection, FormActions, SearchForm, ValidatedForm, SettingsSection, InlineEdit, FilterBar, UploadField, RepeatableFields, FormWizard, AutosaveForm |
| Feedback/state | Alert, Callout, Banner, Badge, Tag, StatusDot, Progress, Meter, Spinner, Skeleton, EmptyState, ErrorState, Toast, ToastRegion, FlashMessages, RequestStatus, ConnectionStatus, JobProgress |
| Navigation | Navbar, SidebarNav, NavGroup, NavItem, Breadcrumbs, Pagination, Tabs, Tab, TabPanel, Steps, TableOfContents, Disclosure, AccountMenu, WorkspaceSwitcher, SearchNavigation |
| Overlays | Dialog, Drawer, Popover, Tooltip, DropdownMenu, MenuItem, MenuSeparator, ConfirmDialog, FormDialog, RemoteDialog |
| Data display | Card, Panel, Avatar, AvatarGroup, Table, TableRow, TableCell, Stat, MetricGrid, Timeline, TimelineItem, Accordion, AccordionItem, ResourceList, ResourceListItem, DetailPanel, ActivityFeed, ActivityItem, FileList, FileItem, ResultCount, SortControl, PageSizeSelect |
| Backend-aware collections | DataTable, DataList, CollectionToolbar, BulkActionBar, LoadMoreList, InfiniteList, RemoteSearch, RemoteSelect, DependentFields, LiveRegion, NotificationCenter, JobList |
| Application composition | PageHeader, SectionHeader, AppShell, DocsShell, MarketingShell, Footer, ThemeSwitcher, SplitPane, MasterDetail |
| Marketing | Section, Hero, Feature, FeatureGrid, CtaBand, ComparisonTable, PricingTier, PricingTiers, Testimonial, TestimonialGroup, LogoCloud, Faq, FaqItem, NewsletterForm, ContactForm |
| Diagrams | Node, NodeLabel, NodeStatus, Connection, ConnectionLabel, Flow, Branch, Layers, Sequence, Compare, Boundary, Topology, DiagramCaption, DiagramLegend, Reveal, Walkthrough, DiagramInspector, LiveDiagram |

Consolidate overlapping names before freezing public API. Examples: File versus FileInput; Accordion/Faq versus Disclosure; ConfirmAction versus ConfirmDialog; RemoteSearch versus SearchForm plus results. Not every inventory item needs its own public Rust type. Prefer shared mechanisms with convenient builders where they serve the same semantics.

Keep complete collection, detail, edit, settings, dashboard, search, authentication, file-manager, job-monitor, documentation, landing, pricing, and error pages as gallery recipes initially.

### Recommended first additions

Prioritize Separator, VisuallyHidden, IconButton, ButtonLink, Image, List, Disclosure, ActionGroup, PageHeader, Panel, ErrorSummary, FormSection, and FilterBar. Build DataTable from CollectionToolbar, Table, EmptyState, Pagination, and result announcements.

Defer autosave, infinite scrolling, live diagrams, job infrastructure, and specialized workflow components until their contracts are demonstrated by an application slice.

## 4. Define backend awareness as presentation plus contracts

Distinguish three modes in documentation:

- **Server:** normal links/forms and full-page rendering.
- **Enhanced:** optional asynchronous requests and partial replacement using the same endpoint semantics.
- **Live:** polling or streaming updates with an initial server-rendered snapshot.

The application owns persistence, query execution, authentication policy, authorization, and business rules. Components receive data, state, and typed endpoint/action descriptions. Rendering must not perform database queries or backend mutations.

Specify loading, validation, success, empty, failure, and conflict states where relevant. No-JavaScript fallback need not reproduce continuous updates, but must offer a usable snapshot and manual refresh/action path. Infinite loading retains a next-page link; autosave retains explicit submission; remote suggestions retain a usable native or server-search path.

## 5. Define shared backend contracts in the framework spec

Use these as a candidate vocabulary. Introduce traits only where multiple implementations or adapters justify them; use concrete structs/enums for state and configuration.

| Contract | Responsibility |
|---|---|
| FromRequest / IntoResponse | Typed HTTP extraction and response conversion; evaluate existing ecosystem contracts before inventing equivalents |
| Endpoint | Generate a URL from typed parameters; pair with an action descriptor containing method and enhancement configuration |
| QueryCodec | Parse and serialize shareable query state |
| CollectionSource | Query typed rows with filtering, sorting, and pagination |
| RecordSource | Fetch a record by typed identity |
| SuggestionSource | Search choices and resolve already-selected IDs |
| FacetSource | Obtain filter choices and optional counts |
| Validate | Return field and form validation errors |
| Action | Execute a typed mutation; convenience adapter over Tower where useful |
| FormModel | Redisplay submitted values and validation state, excluding sensitive values |
| UploadHandler / DownloadSource | Process or produce streamed file content and metadata |
| IdentityProvider / Authorizer | Resolve identity and authorize operations on resources |
| SessionStore / CsrfProtection | Session persistence and CSRF issuance/verification |
| FragmentRenderer / AssetResolver | Render HTML with requirements and resolve deliverable assets; prefer methods on existing types if sufficient |
| FlashStore / ErrorPresenter | Redirect-surviving messages and safe presentation of internal failures |
| EventSource | Typed event stream, with explicitly optional resumption capability |
| JobSource / JobSubmitter / JobCanceller | Observe, start, and request cancellation of background jobs |
| NotificationSource | Notification snapshots and unread counts; mutations use Action |
| IdempotencyStore / Versioned | Optional retry deduplication and revision-based conflict support |

Use a concrete RequestContext for request identity, principal, locale, cancellation, and other per-request metadata. Avoid threading every backend trait as a generic parameter through UI types.

Document these mappings:

- DataTable: CollectionSource + QueryCodec + Endpoint.
- ValidatedForm: extraction + Validate + Action + submitted form state.
- Combobox: SuggestionSource + Endpoint.
- InlineEdit: RecordSource + Action + optional version checking.
- RemoteDialog: endpoint/source + fragment rendering.
- NotificationCenter: snapshot source + Action + optional EventSource.
- JobProgress: JobSource + optional JobCanceller/EventSource.

Decide async Send bounds, ownership, cancellation behavior, and generic versus trait-object support before publishing signatures. Verify current stable Rust and Tower documentation during detailed API design. Do not copy conceptual async-trait sketches directly into public API.

## 6. Make Tower the execution boundary

Represent executable backend operations as typed Tower Services or adapters to them. Do not make visual components implement Service.

Specify how adapters preserve readiness and backpressure. An async execute method alone does not express readiness; adapters must define buffering/concurrency behavior explicitly rather than always reporting ready.

Define the HTTP pipeline and layer placement: tracing/request identity, sessions/identity, input extraction, applicable CSRF checks, resource authorization, validation/action, and response negotiation. Resource authorization often needs extracted IDs or fetched records; do not require every authorization decision to happen in generic middleware.

Separate expected outcomes (validation, missing resource, conflict, denied operation) from infrastructure errors. Map both deliberately to HTTP status and safe user-facing output. Keep input size limits and upload limits at the request boundary.

Cover Send/Sync and lifetime requirements, Service cloning, cancellation, and error conversion. Retry mutations only under an explicit idempotency contract. Explain that streaming response creation and the stream lifetime require separate timeout/cancellation policies.

## 7. Fix fragment rendering and asset delivery (§4.1, §4.4–5, §8)

The current to_html discards assets but is described as suitable for partial responses. Replace that recommendation with an asset-preserving fragment API, conceptually:

```rust
pub struct RenderedFragment {
    pub html: String,
    pub assets: AssetRequirements,
}
```

Keep to_html as an explicitly HTML-only helper. Define a fragment response protocol and client loader that:

1. Resolves required behaviors and dependencies from Bundle.
2. Loads each hashed asset once, including when concurrent fragments request it.
3. Enhances newly inserted elements and handles load failures visibly.
4. Preserves CSP requirements without injecting arbitrary inline scripts.
5. Handles any fragment-specific CSS and icons. In particular, specify how icons absent from the initial page sprite become available after insertion.

In linked delivery, distinguish the already-loaded compiled CSS bundle from behavior modules loaded on demand. For inline delivery, explicitly specify whether dynamic fragment insertion is supported and how its requirements are delivered.

Specify stable IDs across rerenders and collisions across separately rendered fragments. A per-render counter alone is insufficient for several independent fragments inserted into one document. Consider explicit IDs, caller-provided namespaces, or stable instance keys; choose and document one policy.

Define focus restoration, validation focus, polite result announcements, stale/out-of-order response handling, and cancellation. Use the same application operation for full-page and enhanced requests; negotiate response format afterward. State supported interoperability with htmx rather than assuming its swaps solve asset delivery.

## 8. Resolve existing API and spec inconsistencies

- Theme::build returns BuiltTheme, while presets return customizable Theme and Bundle accepts ThemeSource. Specify exactly when validation occurs, including a modified preset; unvalidated custom themes must not silently bypass checks.
- Box<dyn Render> slots conflict with the requirement that every component is Debug. Define a slot wrapper/debug policy or a suitable bound; also specify how borrowed data works with the current 'static slot restriction.
- Diagram NodeKind requires Icon while icons are optional elsewhere. Confirm the intended diagram feature dependency and behavior for iconless kinds.
- The adapter asset-collection guarantee needs runnable examples, especially Display-based template embedding. Show how adapters retain the active rendering context and avoid nested rendering that loses requirements.
- Distinguish escaped attributes from safe attributes. Generic attr must not bypass Href validation or silently accept executable event-handler attributes. Define the escape hatch and its trust boundary.
- Define allowed element/attribute names, behavior when reserved component attributes are overridden, and how data/ARIA attributes are validated.
- Decide whether public runtime ID and class setters append, replace, reject duplicates, or report conflicts consistently.
- The quality section specifies Playwright but build step 5 says Puppeteer. Standardize on one harness, preferably the already-specified Playwright.
- Define supported browsers and test native dialog invokers, popovers, anchor positioning, light-dark(), and container queries against that matrix. Add fallbacks where required to meet declared support.
- Inline delivery is described as suitable for emails. Clarify that email output needs a restricted, email-compatible subset/delivery strategy; arbitrary UI components, scripts, and modern CSS are not guaranteed to work in email clients.
- Define the distinction between required HTML elements and actual public subcomponent types, and reserve names to avoid accidental API bloat.

## 9. Revise build order (§11)

Replace the entirely sequential library-then-framework roadmap with early integration validation:

1. Core rendering, assets, safe markup, fragment requirements, and stable identity contract.
2. Theme generation and contrast validation; minimal layout, typography, and forms.
3. One thin Tower integration serving Bundle assets and full pages.
4. AppShell + PageHeader + server DataTable with GET sorting/filtering/pagination.
5. Edit form with submitted-value redisplay, server validation, CSRF, and authorization.
6. Partial updates using the same endpoints; insert a component whose behavior and icon were absent from the initial page.
7. Expand primitives/compositions and gallery coverage in the existing family order as appropriate.
8. Add uploads, job observation, and one live-update path only after separate framework scope review.
9. Complete marketing, diagrams, adapters, documentation, and release work for the agreed v1 scope.

Each slice includes a working example, no-JavaScript path, keyboard/accessibility checks, and relevant feature-build checks. The thin integration is an intentional change to project sequencing and needs explicit scope in the revised specs.

## 10. Acceptance criteria for the revised specs

- Component-only consumers do not acquire a server/runtime dependency.
- Public composition boundaries, slot semantics, landmark ownership, and feature dependencies are documented.
- Each backend-aware component identifies its request/state contracts and fallback.
- Full-page and partial modes share operation semantics and preserve validation/query state.
- Fragments can introduce previously unused behavior modules and icons without duplicate loading or ID collisions.
- Tower adapters honor readiness and document cancellation, Send bounds, and error mapping.
- Authorization is enforced at execution; CSRF checks cover applicable cookie-authenticated mutations.
- Integration tests cover validation redisplay, conflict handling where supported, asset introduction, stale responses, focus, and no-JavaScript submission.
- Feature tests include minimal builds, selected combinations, and all features; avoid checking only the default bundle.
- The roadmap clearly separates committed v1 scope from later catalog items.
- No unresolved contradiction is hidden behind a proposed trait name or an illustrative code snippet.

## Expected agent deliverable

Produce the revised library spec, a clearly bounded framework integration proposal/spec as appropriate to authorized scope, and a concise decision list. Explain material scope changes and remaining choices. Keep implementation plans and product code out of this spec-revision task.
