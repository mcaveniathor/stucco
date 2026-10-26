// Public credentials provisioned only inside the disposable browser-test database.
export const accounts = {
  alice: { username: "alice", password: "browser-test-alice-password", tenant: "team-a", displayName: "Alice" },
  bob: { username: "bob", password: "browser-test-bob-password", tenant: "team-b", displayName: "Bob" },
  carol: { username: "carol", password: "browser-test-carol-password", tenant: "team-a", displayName: "Carol" },
  export_chromium: { username: "export-chromium", password: "browser-test-export-chromium", tenant: "team-a", displayName: "Chromium exports" },
  export_firefox: { username: "export-firefox", password: "browser-test-export-firefox", tenant: "team-a", displayName: "Firefox exports" },
  export_webkit: { username: "export-webkit", password: "browser-test-export-webkit", tenant: "team-a", displayName: "WebKit exports" },
};
