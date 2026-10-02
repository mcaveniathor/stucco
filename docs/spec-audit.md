# Admin application specification: audit

Status of each requirement in the admin application feature specification
(version 1.0, 2026-10-02), checked against this repository after the
"complete CRUD workflows" change. **Done** means a public API or documented
recipe covers it and a test exercises it. **Partial** means part is covered
and the note says what is missing. **Not yet** means nothing covers it.

The acceptance tests for Priority 1 are the orders example
(`examples/orders`) and its tests: `examples/orders/tests/http.rs` drives
every workflow through the router, and `browser/tests/orders.spec.ts` runs
create → invalid submission → save → edit → archive → delete without
JavaScript and checks the order pages with axe at 320px.

## 4. Forms (Priority 1)

| Requirement | Status | Where / note |
| --- | --- | --- |
| Field wrapper with stable ids, label, help text, errors | Done | `Field`; ids are stable when set with `.id()`, otherwise generated per document (unique, but order-dependent) |
| Text, email, password, number, textarea, select, checkbox, radio | Done | `Input::*`, `Textarea`, `Select`, `Checkbox`, `RadioGroup` |
| Fieldset and legend | Done | `Fieldset`, `Legend` |
| Required, optional, disabled, read-only | Partial | `Field::required`/`optional`; `disabled`/`readonly` on `Input` and `Textarea`, `disabled` on `Select` and `Checkbox`. `RadioGroup` has no `disabled` |
| Error summary linking to controls | Done | `ErrorSummary` (takes focus on load) |
| Form-level errors for backend failures and conflicts | Done | `FormState::with_form_error`; guide table of 422/409/500; example returns 409 and 500 with values kept |
| Submitted values preserved, including invalid raw input | Done | `FormState` keeps raw text; `Field::bind`; tested with `12.x` and an impossible date |
| Repeated values, missing checkboxes, malformed input | Done | `Validator::text` (first), `single` (refuses repeats), `list`, `flag` |
| Initial values distinguished from submitted | Done | `FormState::is_submitted` |
| Save/cancel action group | Done | `FormActions` |
| POST → redirect → GET with feedback | Done | `SeeOther::flash`, `IncomingFlash`, `Notice`; forms guide "Feedback after a save" |
| Guidance on secrets in repopulation and logs | Done | Sensitive controls never render values; `FormState::without_values`; forms guide "Passwords and secrets" |
| Adapter hooks for hidden application tokens | Partial | `Form::csrf`/`Confirmation::csrf` render `_csrf`, `FormState` drops it, `behavior::HEADER_CSRF` names a header. Issuing and verifying is the application's (by design); no adapter middleware ships |

Acceptance criterion: met by the orders example.

## 5. Collections and tables (Priority 1)

| Requirement | Status | Where / note |
| --- | --- | --- |
| Caption, header cells, scope | Done | `Table`, `DataTable` |
| Typed columns: text, numbers, dates, status, links, row actions | Done | `Col::text`/`number`/`date`/`enumeration`/`custom`, `Col::href`, `Col::actions`, `#[col(link = …)]` |
| Server-driven sorting, direction shown visually and with `aria-sort` | Done | `DataTable` headings |
| Search and filters from declared capabilities | Done | `Capabilities`, `FilterBar` |
| Known-total and cursor pagination | Done | `Pagination` |
| Sorting and pagination keep active parameters | Done | `CollectionQuery::link`; HTTP and unit tests |
| Changing filters resets the position | Done | `with_search`/`with_filter`/`with_sort`/`with_per_page` |
| Individual filter removal and clear-all | Done | `ActiveFilters` |
| Distinct empty, no-match and backend-error states | Done | `DataTable` empty states and `empty(...)`; a failed load is a `Notice` with status 500 (collections guide) |
| Result counts that don't invent totals | Done | `ResultCount` |
| Stable row ids and clear row-action labels | Done | `DataTable::row_id`; guidance on naming actions per row |
| Responsive overflow or compact record presentation | Partial | Tables scroll horizontally in a labelled, focusable region; no stacked/card presentation for narrow screens |
| Right-aligned numbers, readable long cells | Done | `data-kind="number"`, `Col::wrap` |
| Permission-aware actions supplied by the application | Done | `Col::actions` closure decides per row; example omits Edit for archived orders and refuses it in the handler |
| Selection and bulk actions with native forms | Done | `DataTable::selectable` (checkboxes join a form by `form` attribute) |
| Bulk confirmation names action and scope | Done | Example's bulk confirmation via `Confirmation` |
| Query contract (serialization, operators, bounds, unsupported parameters) | Done | Collections guide "The query contract" |
| Pagination without loading the whole dataset; adapter costs | Done | Cursor mode; costs in the collections guide and the orders README |
| Selection contract (explicit ids, rechecked; no implied select-all) | Done | Collections guide; example rechecks each id |

Acceptance criterion: met. Tests cover parameter preservation, pagination
reset, unsupported and malformed parameters, unknown totals, and returning
from a record to the same list.

## 6. Application navigation (Priority 1)

| Requirement | Status | Where / note |
| --- | --- | --- |
| Shell with header, sidebar, main landmark, skip link | Done | `AppShell` |
| Active navigation with `aria-current` | Done | `NavLink::current` |
| Breadcrumbs | Done | `Breadcrumbs`, `PageHeader::breadcrumbs` |
| Nested navigation with controlled disclosure | Partial | The sidebar is a disclosure; no nested navigation group component |
| Account actions and organization/project switching | Not yet | Can be composed from `Menu`; no dedicated component |
| Responsive navigation without JavaScript | Done | Shell layout adapts with container queries |
| Clear page and section headings | Done | `PageHeader`, `SectionHeader` |
| Page actions that wrap at narrow widths | Done | `PageHeader` actions wrapper |

Acceptance criterion: partly verified. Automated: axe and no horizontal
scroll at 320px on every orders page, skip link by keyboard. Manual
screen-reader passes are still to do.

## 7. Page compositions (Priority 1)

| Composition | Status | Note |
| --- | --- | --- |
| Collection page | Partial | Composed in the example from `PageHeader` and `DataTable`; not extracted |
| Record detail | Partial | `PageHeader`, `StatusBadge`, `DescriptionList`, `ActivityList`; composed in the example |
| Create/edit page | Partial | `ErrorSummary`, `Fieldset`, `Field`, `FormActions`; composed in the example |
| Confirmation page | Done | `Confirmation` |
| Settings page | Not yet | |
| Result pages (missing, access denied, expired session, failure) | Partial | Example has not-found (404) and failure (500) pages from `EmptyState` and `Notice`; no library composition, no access-denied or expired-session example |
| Activity page | Partial | `ActivityList`; no page composition |

| Requirement | Status | Note |
| --- | --- | --- |
| Full-page confirmations without JavaScript | Done | `Confirmation` |
| Feedback and a predictable return location after a change | Done | Flash + `back` parameter in the example |
| Archive and permanent deletion visibly distinct | Done | Different pages, wording and button variants |
| Return URLs validated by the application | Done | Example's `local_return`; guidance in forms and collections guides |
| Any section replaceable with custom content | Done | Every slot takes any `Render` |

Compositions beyond `Confirmation` were deliberately not extracted yet: the
specification's sequence says to extract them once the example shows
repetition, and one resource doesn't.

## 8. Operational and settings UI (Priority 2)

| Requirement | Status | Note |
| --- | --- | --- |
| Status badge, text first | Done | `StatusBadge` |
| Description/key-value list | Done | `DescriptionList` |
| Timestamp with machine value and time zone | Done | `Timestamp` (no formatting or zone conversion: the application supplies both forms) |
| Duration and expiration | Not yet | |
| Copyable identifiers | Not yet | Identifiers render as selectable text; no copy button |
| Activity timeline | Done | `ActivityList` |
| Metric card | Not yet | |
| Success, warning, error and info notices | Done | `Notice` |
| Settings patterns (sectioned forms, members, API keys, one-time secrets, integrations, organization switching) | Not yet | |

## 9. Progressive enhancement (Priority 2)

| Requirement | Status | Note |
| --- | --- | --- |
| Copy buttons | Not yet | |
| Responsive sidebar interaction | Done | Native disclosure, no script |
| Dialogs with focus handling and a full-page alternative | Done | `Dialog`, `Dialog::link_opener` |
| Menus with keyboard operation and focus restoration | Done | `Menu` (popover) |
| Submission feedback and duplicate-click reduction | Not yet | `Button::loading` renders a busy state; nothing sets it on submit |
| Fragment updates from the same server-rendered content | Partial | Fragment negotiation (`PageCx::respond`, `render_fragment`) and module loading exist; the runtime doesn't swap fragments, manage focus or history |
| Focus, announcements and history after updates | Not yet | |
| Recoverable network failures | Partial | A failed module load marks its region and dispatches `stucco:asset-error` |
| Polling with freshness and cleanup | Not yet | |
| Defined status, redirect and error behaviour for fragments | Partial | `Vary` and `no-store` on fragments; `Stucco-Location` header defined; behaviour per status is not yet documented end to end |

## 10. Priority 3

Not started, as the specification asks.

## 11. Cross-cutting

| Requirement | Status | Note |
| --- | --- | --- |
| Names, landmarks, heading order, descriptions | Done for shipped components | axe on every gallery page and orders page |
| Visible focus and keyboard operation | Done | `:focus-visible` rings; native controls |
| Errors associated with fields and summaries | Done | `aria-describedby`, `aria-invalid`, `ErrorSummary` |
| Announcements without noise | Done | `Notice` announces outcomes, `quiet()` for standing notices |
| Reduced motion | Done | Theme motion tokens respect `prefers-reduced-motion` |
| Narrow widths and larger text | Partial | No horizontal scroll at 320px on orders pages; text-zoom not tested automatically |
| Manual keyboard and screen-reader inspection | Not yet | Documented as needed (components guide) |
| Escaped text and attributes | Done | Escaping by default; `Raw::trusted` is the explicit escape hatch |
| Consistent URL validation | Done | `Href` |
| Unique generated ids | Done | Duplicate ids panic in debug builds |
| No mutation through GET | Done | Example routes changes as POST; tested (405 / confirmation only) |
| Meaningful HTTP statuses | Done | 303, 404, 409, 422, 500 in the example; `Document::status` |
| Security obligations documented | Done | Forms guide (CSRF, secrets, return URLs), collections guide (permissions, cursors), orders README |
| Copyable workflow recipes | Done | Recipes: CRUD routes, confirmations; forms guide |
| Empty, error, disabled and long-content examples | Done | Gallery Forms, Records and feedback, and Collections pages |
| Query and form contracts documented | Done | Collections and forms guides |
| Feature selection guidance | Done | README feature table |
| Compatibility policy and migration notes | Partial | CHANGELOG notes compatibility per release; no written policy |
| Implemented vs reserved features | Done | README lists reserved feature names |

## 12–13. Commercial starter

Out of scope for the open-source library; nothing here is premium. The
orders example's README lists what a real application adds: sign-in,
permissions, CSRF tokens, migrations and a scalable write path.

## Suggested next steps

1. A narrow-screen record presentation for `DataTable` (rows as stacked
   property lists below a width).
2. `RadioGroup::disabled`, and nested navigation groups for `AppShell`.
3. Submission feedback (busy state on submit, duplicate-click reduction) and
   copy buttons as small behaviours.
4. A second resource in an example, then extract the collection, detail and
   form page compositions that repeat.
5. Settings patterns, starting with sectioned settings forms and API keys
   with one-time secret display.
