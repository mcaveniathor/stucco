use stucco_core::{Cursor, Direction, Window};
use stucco_redb::{PostcardCodec, ScanRequest, Store, U64Key};
fn request(direction: Direction, window: Window) -> ScanRequest {
    ScanRequest {
        index: Some("name".into()),
        direction,
        window,
        per_page: 2,
        scope: "all".into(),
        offset_mode: false,
    }
}
#[test]
fn forward_backward_and_deleted_anchors_preserve_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("scan.redb")).unwrap();
    let table = store
        .table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("name", |v: &String| v.as_bytes().to_vec())
        .unwrap();
    table
        .put_many(
            &(1..=7)
                .map(|id| (id, format!("{}-{id}", if id <= 3 { "Ada" } else { "Zed" })))
                .collect::<Vec<_>>(),
        )
        .unwrap();
    for direction in [Direction::Asc, Direction::Desc] {
        let first = table
            .scan(&request(direction, Window::default()), |_| true)
            .unwrap();
        assert!(first.prev.is_none());
        let second = table
            .scan(
                &request(direction, Window::After(first.next.clone().unwrap())),
                |_| true,
            )
            .unwrap();
        let back = table
            .scan(
                &request(direction, Window::Before(second.prev.unwrap())),
                |_| true,
            )
            .unwrap();
        assert_eq!(back.rows, first.rows);
        let mut rows = first.rows;
        rows.extend(second.rows);
        let mut cursor = second.next;
        while let Some(c) = cursor {
            let page = table
                .scan(&request(direction, Window::After(c)), |_| true)
                .unwrap();
            rows.extend(page.rows);
            cursor = page.next;
        }
        let mut expected: Vec<_> = (1..=7)
            .map(|id| format!("{}-{id}", if id <= 3 { "Ada" } else { "Zed" }))
            .collect();
        if direction == Direction::Desc {
            expected.reverse();
        }
        assert_eq!(rows, expected);
    }
    let first = table
        .scan(&request(Direction::Asc, Window::default()), |_| true)
        .unwrap();
    table.remove(&2).unwrap();
    assert_eq!(
        table
            .scan(
                &request(Direction::Asc, Window::After(first.next.unwrap())),
                |_| true
            )
            .unwrap()
            .rows[0],
        "Ada-3"
    );
}
#[test]
fn filtering_stale_cursors_and_extreme_offsets_are_safe() {
    let dir = tempfile::tempdir().unwrap();
    let table = Store::open(dir.path().join("filter.redb"))
        .unwrap()
        .table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("name", |v: &String| v.as_bytes().to_vec())
        .unwrap();
    table
        .put_many(&(1..=7).map(|id| (id, id.to_string())).collect::<Vec<_>>())
        .unwrap();
    let keep = |v: &String| v.parse::<u64>().unwrap() % 2 == 1;
    let first = table
        .scan(&request(Direction::Asc, Window::default()), keep)
        .unwrap();
    assert_eq!(first.rows, ["1", "3"]);
    let second = table
        .scan(
            &request(Direction::Asc, Window::After(first.next.clone().unwrap())),
            keep,
        )
        .unwrap();
    assert_eq!(second.rows, ["5", "7"]);
    assert!(second.next.is_none());
    let mut stale = request(Direction::Asc, Window::After(first.next.unwrap()));
    stale.scope = "other".into();
    assert_eq!(table.scan(&stale, keep).unwrap().rows, first.rows);
    stale.window = Window::After(Cursor::new("garbage").unwrap());
    assert_eq!(table.scan(&stale, keep).unwrap().rows, first.rows);
    stale.offset_mode = true;
    stale.window = Window::Offset { page: u64::MAX };
    let page = table.scan(&stale, keep).unwrap();
    assert!(page.rows.is_empty());
    assert_eq!(page.total, Some(4));
    assert!(
        table
            .scan(&request(Direction::Asc, Window::default()), |_| false)
            .unwrap()
            .rows
            .is_empty()
    );
}
