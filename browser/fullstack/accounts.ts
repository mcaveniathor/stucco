import { accounts as provisionedAccounts } from "./accounts.mjs";

export type AccountName = "alice" | "bob" | "carol" | "export_chromium" | "export_firefox" | "export_webkit";
export const accounts: Readonly<Record<AccountName, {
  username: string; password: string; tenant: string; displayName: string;
}>> = provisionedAccounts;
