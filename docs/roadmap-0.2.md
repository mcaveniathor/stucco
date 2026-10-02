# stucco 0.2 plan: writes and overlays

Status: proposal. 0.1 renders and queries data. 0.2 should let an
application change it: a server-rendered create/edit/delete flow that works
without JavaScript, plus native dialogs for confirmations. The orders example
becomes a full CRUD admin and serves as the acceptance test.

## What already exists

- `FormState` (stucco-core) holds submitted values, field errors and form
  errors. `Field::bind`, `Checkbox::bind` and `RadioGroup::bind` render
  from it.
- `Form::post`, `Form::csrf` (`_csrf` hidden input), `Form::multipart`,
  `FieldError`, `FieldHint`, `Fieldset`.
- `stucco-tower`: standard layers (body limit, timeout, request ids),
  `RequestContext`, `PageResponse`/`FragmentResponse`, `RequestKind`
  negotiation.
- `stucco-redb`: `Table::get/put/remove`, transactional secondary indexes.
- The `--st-z-overlay` token and `Level::Overlay` surface.

The missing pieces are parsing a submission, validation, CSRF issuing and
verifying, post-redirect-get with flash messages, storage writes from async
handlers, and overlay components.

## Milestone 1: form submissions (stucco-core, stucco-tower)

1. **Parse a submission into `FormState`.** `FormState::from_urlencoded(&[u8])`
   in core, which keeps repeated keys and drops `_csrf`. Add an axum extractor,
   `Submission`, in stucco-tower behind the `axum` feature. It enforces
   `application/x-www-form-urlencoded` and the body limit, and returns
   `415`/`413` otherwise.
2. **Validation.** Keep it small and dependency-free: a `Validator` builder over
   `FormState` (`required`, `max_len`, `parse::<T>`, `one_of`, a custom closure)
   that records messages on the state and yields typed values only if
   everything passes. Users who prefer `validator`/`garde` can still fill
   `FormState` by hand.
   - Open question: add a derive (`#[derive(FromForm)]`)? Recommendation: not
     in 0.2. A proc-macro crate adds compile-time and MSRV cost; the builder
     shows whether the shape is right first.
3. **Error summary.** An `ErrorSummary` component (forms family) that lists
   `form_errors()` and links each field error to its control (`href="#id"`).
   It gets focus on load via `autofocus` on a `tabindex="-1"` heading, with no
   JavaScript.
4. **Responses.** `SeeOther` (303) in stucco-tower for post-redirect-get. When
   the state has errors, re-render the page with `422 Unprocessable Content`
   and the bound `FormState`.

## Milestone 2: CSRF and flash messages (stucco-tower)

1. **CSRF.** Use a signed double-submit cookie. `CsrfLayer` sets an
   `HttpOnly; SameSite=Lax; Secure` cookie holding a random token. It puts the
   form token (HMAC of the cookie token with the app key) into `RequestContext`
   so handlers can pass it to `Form::csrf`. It rejects unsafe methods with
   `403` unless `_csrf` (or an `x-csrf-token` header) verifies. The handler
   reads the token with `cx.get::<CsrfToken>()`.
   - Also check the `Origin`/`Sec-Fetch-Site` headers as defence in depth.
   - New deps: `hmac`, `sha2`, `getrandom` (or `rand`). Put them behind a
     `csrf` feature, on by default.
2. **Flash.** `Flash` holds a one-shot message and level (info/success/warning/
   error) in a signed, short-lived cookie that is cleared on read. The
   `FlashMessages` component (feedback family) renders it in the existing
   `LiveRegion` with `role="status"` (or `role="alert"` for errors).
3. **Keys.** `stucco_tower::SecretKey`: generated in development with a logged
   warning, required in release builds (a startup error if missing).

## Milestone 3: async writes (stucco-redb)

- An async write path to match the read adapter: `AsyncTable::put/remove/
  update` via `spawn_blocking`, with secondary indexes maintained in the same
  transaction (they already are for synchronous writes).
- `update(key, |old| -> Option<new>)` for read-modify-write in one
  transaction, so two edits can't silently overwrite each other.
- Optimistic concurrency, optional: a `version` field the edit form carries as
  a hidden input; a mismatch becomes a form error ("This order changed while
  you were editing").

## Milestone 4: overlay family (stucco-ui `overlay` feature)

All of these use native platform features and need no runtime JavaScript
beyond what the browser provides:

| Component | Mechanism | Notes |
| --- | --- | --- |
| `Popover` | `popover` attribute and `popovertarget` | Menus, help text |
| `Menu` | `Popover` with a list of `ButtonLink`/`Button` | Row actions in `DataTable` |
| `Dialog` | `<dialog>` opened with `commandfor`/`command="show-modal"` | Behaviour module fallback for browsers without invoker commands |
| `ConfirmAction` | `Dialog` wrapping a POST `Form` | Delete confirmation; without JavaScript, the trigger links to a full-page confirm route |

- Every overlay needs labelled titles (`aria-labelledby`), focus return and
  Escape to close; add Playwright checks in `browser/tests/overlay.spec.ts`
  across all three engines.
- Add a gallery page and snapshot for the family.

## Milestone 5: orders example becomes CRUD

- `GET /orders/new`, `POST /orders`, `GET /orders/{id}/edit`,
  `POST /orders/{id}`, `POST /orders/{id}/delete` (HTML forms cannot send
  PUT/DELETE; no method-override magic).
- A `DataTable` row-action `Menu` with Edit and Delete (Delete opens
  `ConfirmAction`).
- Integration tests in `examples/orders/tests/http.rs`: a valid create
  redirects 303 and shows a flash; an invalid one returns 422 with the values
  kept and errors linked; a missing or wrong CSRF token returns 403; a stale
  version shows a conflict error.
- Browser tests run the whole flow once with JavaScript enabled and once
  disabled.

## Breaking changes to bundle into 0.2

0.2 is allowed to break the API, so collect the planned breaks here:

- Remove `overlay` from the default features until the family ships, then put
  it back. If Milestone 4 ships in 0.2, just keep it.
- Remove the reserved feature names that have no 0.2 implementation
  (`marketing`, `diagram`, `markdown`, `askama`, `maud`). Re-adding a feature
  later is not breaking; keeping empty ones invites dependents on nothing.
- Review the `FormState` builder names (`with_value`/`with_error`) against the
  new validator API so the two read consistently.

## Out of scope for 0.2

Authentication, sessions beyond flash and CSRF, file upload handling (beyond
`Form::multipart` markup), schema migrations, fragment swapping and live
search, and new storage adapters. Those are candidates for 0.3.

## Suggested order and size

1 → 2 → 3 can be reviewed independently and in parallel with 4. Milestone 5
needs all four. Roughly one PR per milestone, with Milestone 4 likely split into
popover/menu and dialog/confirm.
