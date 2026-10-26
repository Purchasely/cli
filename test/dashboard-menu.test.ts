import { EventEmitter } from "node:events";
import { describe, expect, it, vi } from "vitest";
import { EXT, setup, type Call, type Reply } from "./helpers.js";

/**
 * Fake terminal. `scripts` has one list of keys per `play()` call: each list is typed when that menu starts to read,
 * so a key never arrives before its listener.
 */
function rig(scripts: string[][], tty = true, columns = 100) {
  const queue = [...scripts];
  const stdin = Object.assign(new EventEmitter(), {
    isTTY: tty,
    setRawMode: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(() => {
      const keys = queue.shift();
      if (keys) setTimeout(() => keys.forEach((name) => stdin.emit("keypress", undefined, { name })), 0);
    }),
  });
  const written: string[] = [];
  const stdout = Object.assign(new EventEmitter(), { isTTY: tty, columns, rows: 24, write: (s: string) => written.push(s) > 0 });
  const proc = new EventEmitter();
  return { stdin, stdout, proc, exit: vi.fn(), written, io: { stdin, stdout, proc, exit: vi.fn() } as any };
}

const apps = { body: { mobile_applications: [{ id: "app_1", name: "One" }, { id: "app_2", name: "Two" }] } };
const replies = (c: Call): Reply => (new URL(c.url).pathname === "/management/v1/applications" ? apps : { body: {} });
const paths = (t: { calls: Call[] }) => t.calls.map((c) => new URL(c.url).pathname);
const FILLED = ["--app", "--date-range", "--periodicity"];
const downs = (n: number) => Array.from({ length: n }, () => "down");

/** The list as an agent reads it. */
async function list() {
  const t = setup({ replies });
  t.d.io = rig([], false).io;
  const r = await t.exec("dashboard");
  return r.json() as { command: string; description: string; required_flags: string[] }[];
}

describe("dashboard with no subcommand", () => {
  it("not in a terminal: prints the list as JSON, summary first, and sends no request", async () => {
    const t = setup({ replies });
    t.d.io = rig([], false).io;
    const r = await t.exec("dashboard");
    expect(r.code, r.err).toBe(0);
    expect(t.calls).toHaveLength(0);
    const rows = r.json();
    expect(rows[0]).toEqual({ command: "purchasely dashboard summary", description: expect.stringContaining("Analytics dashboard"), required_flags: ["--app"] });
    expect(rows.map((x: { command: string }) => x.command)).toEqual(expect.arrayContaining(["purchasely dashboard header-live", "purchasely dashboard conversion", "purchasely dashboard mau-reports"]));
    expect(rows.every((x: { description: string }) => x.description)).toBe(true);
    const conversion = rows.find((x: { command: string }) => x.command === "purchasely dashboard conversion");
    expect(conversion.required_flags).toEqual(["--app", "--date-range"]);
    // --app is optional where the spec has a global form: it is not a required flag.
    expect(rows.find((x: { command: string }) => x.command === "purchasely dashboard mau-reports").required_flags).toEqual([]);
  });

  it("--output json in a terminal prints the same list and opens no menu", async () => {
    const r = rig([["return"]]);
    const t = setup({ replies });
    t.d.io = r.io;
    const out = await t.exec("dashboard", "--output", "json");
    expect(out.code, out.err).toBe(0);
    expect(out.json()).toEqual(await list());
    expect(t.calls).toHaveLength(0);
    expect(r.stdin.setRawMode).not.toHaveBeenCalled();
  });

  it("another program lists the same dashboards under its own name", async () => {
    const t = setup({ replies, name: "acme", ext: EXT });
    t.d.io = rig([], false).io;
    const rows = (await t.exec("dashboard")).json();
    expect(rows[0].command).toBe("acme dashboard summary");
  });

  it("menu: asks the app, then runs summary for the last 30 days as a table, and prints the command line on stderr", async () => {
    const r = rig([["down", "return"], ["return"]]);
    const t = setup({ replies });
    t.d.io = r.io;
    const out = await t.exec("dashboard");
    expect(out.code, out.err).toBe(0);
    expect(paths(t)).toEqual([
      "/management/v1/applications",
      "/management/v1/applications/app_2/dashboard/mrr_evolution_live",
      "/management/v1/applications/app_2/dashboard_v2/subscription_evolution",
      "/management/v1/applications/app_2/dashboard_v2/conversion",
    ]);
    expect(out.err).toContain("purchasely dashboard summary --app app_2 --days 30 --output table\n");
    expect(out.out).toMatch(/^app_2 {2}2023-10-16 to 2023-11-14 {2}\(DAY\)/);
    expect(r.stdin.setRawMode.mock.calls).toEqual([[true], [false], [true], [false]]);
    expect(r.written.join("")).toContain("Two");
  });

  it("menu with --app: no app menu, the chosen dashboard gets the last 30 days and prints JSON", async () => {
    const at = (await list()).filter((x) => x.required_flags.every((f) => FILLED.includes(f))).findIndex((x) => x.command === "purchasely dashboard conversion");
    const r = rig([[...downs(at), "return"]]);
    const t = setup({ replies });
    t.d.io = r.io;
    const out = await t.exec("dashboard", "--app", "app_1", "--env", "staging", "--reason", "look at it");
    expect(out.code, out.err).toBe(0);
    expect(t.calls).toHaveLength(1);
    expect(t.calls[0].url).toBe("https://api-staging.purchasely.io/management/v1/applications/app_1/dashboard_v2/conversion?date_range[]=2023-10-16&date_range[]=2023-11-14");
    expect(t.calls[0].init.headers["User-Agent"]).toContain("look at it");
    expect(out.err).toContain('purchasely dashboard conversion --app app_1 --date-range 2023-10-16,2023-11-14 --env staging --reason "look at it"\n');
    expect(out.json()).toEqual({});
    expect(r.stdin.setRawMode.mock.calls).toEqual([[true], [false]]);
  });

  it("a dashboard without a date range gets none", async () => {
    const at = (await list()).filter((x) => x.required_flags.every((f) => FILLED.includes(f))).findIndex((x) => x.command === "purchasely dashboard mau-reports");
    const t = setup({ replies });
    t.d.io = rig([[...downs(at), "return"]]).io;
    const out = await t.exec("dashboard", "--app", "app_1");
    expect(out.code, out.err).toBe(0);
    expect(t.calls[0].url).toBe("https://api.purchasely.io/management/v1/applications/app_1/dashboard_v2/mau_reports");
    expect(out.err).toContain("purchasely dashboard mau-reports --app app_1\n");
  });

  it("quit restores the terminal and sends no request, with --app", async () => {
    const r = rig([["q"]]);
    const t = setup({ replies });
    t.d.io = r.io;
    const out = await t.exec("dashboard", "--app", "app_1");
    expect(out.code, out.err).toBe(0);
    expect(t.calls).toHaveLength(0);
    expect(r.stdin.setRawMode.mock.calls).toEqual([[true], [false]]);
    expect(r.written.at(-1)).toContain("\x1b[?1049l");
    expect(r.stdin.listenerCount("keypress")).toBe(0);
    expect(r.stdout.listenerCount("resize")).toBe(0);
    expect(r.proc.listenerCount("SIGTERM")).toBe(0);
    expect(out.out).toBe("");
  });

  it("quit in the app menu stops after the one apps request", async () => {
    const r = rig([["q"]]);
    const t = setup({ replies });
    t.d.io = r.io;
    const out = await t.exec("dashboard");
    expect(out.code, out.err).toBe(0);
    expect(paths(t)).toEqual(["/management/v1/applications"]);
    expect(r.stdin.setRawMode.mock.calls).toEqual([[true], [false]]);
  });

  it("the app request goes through the normal path: an API error keeps its exit code, and no key is exit 3", async () => {
    const t = setup({ replies: [{ status: 403, body: { error: { code: "FORBIDDEN", messages: ["no scope"] } } }] });
    t.d.io = rig([]).io;
    expect((await t.exec("dashboard")).code).toBe(4);
    const n = setup({ env: { PURCHASELY_API_KEY: "" } });
    n.d.io = rig([]).io;
    expect((await n.exec("dashboard")).code).toBe(3);
    expect(n.calls).toHaveLength(0);
  });

  it("--dry-run needs --app, because the app menu sends a request", async () => {
    const t = setup({ replies });
    t.d.io = rig([]).io;
    const out = await t.exec("dashboard", "--dry-run");
    expect(out.code).toBe(2);
    expect(out.errJson().error.messages[0]).toContain("--dry-run needs --app");
    expect(t.calls).toHaveLength(0);
  });

  it("no application for the key is a not found error", async () => {
    const t = setup({ replies: [{ body: { mobile_applications: [] } }] });
    t.d.io = rig([]).io;
    const out = await t.exec("dashboard");
    expect(out.code).toBe(6);
    expect(out.errJson().error.messages).toEqual(["No application found for this key."]);
  });

  it("an option before a subcommand is refused with exit 2 and sends no request", async () => {
    for (const before of [["--dry-run"], ["--env", "staging"], ["--output", "table"]]) {
      const t = setup({ replies });
      const out = await t.exec("dashboard", ...before, "summary", "--app", "app_1");
      expect(out.code, before.join(" ")).toBe(2);
      expect(out.errJson().error.messages[0]).toBe("Put the options after the subcommand: purchasely dashboard summary --app ...");
      expect(t.calls).toHaveLength(0);
    }
    // The menu path keeps the options of the group.
    const t = setup({ replies });
    t.d.io = rig([["q"]]).io;
    expect((await t.exec("dashboard", "--env", "staging", "--app", "app_1")).code).toBe(0);
  });

  it("dashboard help summary prints the help of summary", async () => {
    const t = setup({ replies });
    const out = await t.exec("dashboard", "help", "summary");
    expect(out.code, out.err).toBe(0);
    expect(out.out).toContain("Usage: purchasely dashboard summary");
    expect(t.calls).toHaveLength(0);
  });

  it("two apps whose cut lines are equal: the chosen one runs, not the first with the same text", async () => {
    const same = { body: { mobile_applications: [{ id: "app_1", name: "The same long application name 1" }, { id: "app_2", name: "The same long application name 2" }] } };
    const t = setup({ replies: (c) => (new URL(c.url).pathname === "/management/v1/applications" ? same : { body: {} }) });
    t.d.io = rig([["down", "return"], ["return"]], true, 30).io;
    const out = await t.exec("dashboard");
    expect(out.code, out.err).toBe(0);
    expect(paths(t)[1]).toBe("/management/v1/applications/app_2/dashboard/mrr_evolution_live");
  });

  it("an app name with control codes is drawn without them", async () => {
    const evil = { body: { mobile_applications: [{ id: "app_1", name: "Evil\x1b[2JApp\nX" }] } };
    const r = rig([["q"]]);
    const t = setup({ replies: [evil] });
    t.d.io = r.io;
    expect((await t.exec("dashboard")).code).toBe(0);
    const text = r.written.join("");
    expect(text).not.toContain("\x1b[2J");
    expect(text).toContain("Evil [2JApp X");
  });

  it("the app ids line up in one column", async () => {
    const mixed = { body: { mobile_applications: [{ id: "app_1", name: "Short" }, { id: "app_2", name: "A much longer name" }] } };
    const r = rig([["q"]]);
    const t = setup({ replies: [mixed] });
    t.d.io = r.io;
    expect((await t.exec("dashboard")).code).toBe(0);
    const rows = r.written.join("").replace(/\x1b\[[\d;?]*[A-Za-z]/g, "").split("\r\n");
    const at = (id: string) => rows.find((l) => l.includes(id))!.indexOf(id);
    expect(at("app_1")).toBeGreaterThan(0);
    expect(at("app_1")).toBe(at("app_2"));
  });

  it("a resize closes the menu with a message on stderr and exit 0", async () => {
    const r = rig([]);
    r.stdin.resume = vi.fn(() => void setTimeout(() => r.stdout.emit("resize"), 0));
    const t = setup({ replies });
    t.d.io = r.io;
    const out = await t.exec("dashboard", "--app", "app_1");
    expect(out.code, out.err).toBe(0);
    expect(out.err).toContain("The terminal changed size.");
    expect(t.calls).toHaveLength(0);
    expect(r.stdin.setRawMode.mock.calls).toEqual([[true], [false]]);
  });

  it("the menu leaves out a dashboard with a required flag it cannot fill, the JSON list keeps it", async () => {
    const all = await list();
    const funnel = all.find((x) => x.command === "purchasely dashboard funnel")!;
    expect(funnel.required_flags).toContain("--dimension");
    expect(all.find((x) => x.command === "purchasely dashboard flow-paths")!.required_flags).toContain("--flow-id");
    const fillable = all.filter((x) => x.required_flags.every((f) => FILLED.includes(f)));
    expect(fillable.length).toBeLessThan(all.length);
    // Down once per menu entry wraps to the first one, summary. With the full list the cursor would stop on another dashboard.
    const t = setup({ replies });
    t.d.io = rig([[...downs(fillable.length), "return"]]).io;
    const out = await t.exec("dashboard", "--app", "app_1");
    expect(out.code, out.err).toBe(0);
    expect(out.err).toContain("purchasely dashboard summary --app app_1 --days 30 --output table\n");
  });

  it("the subcommands keep their own options", async () => {
    const t = setup({ replies });
    expect((await t.exec("dashboard", "summary", "--app", "app_1", "--output", "table")).code).toBe(0);
    expect(paths(t)[0]).toBe("/management/v1/applications/app_1/dashboard/mrr_evolution_live");
    expect((await t.exec("dashboard", "header-live", "--app", "app_1", "--date-range", "2026-01-01,2026-01-31")).code).toBe(0);
  });

  it("the menu scrolls: every entry fits the screen and the last one can be reached", () => {
    const names = Array.from({ length: 30 }, (_, i) => `entry-${i}`);
    return import("../src/games/term.js").then(({ menu }) => {
      const g = menu(names, { title: "Dashboards", verb: "run" })(100, 20);
      for (let i = 0; i < 29; i++) g.key("down");
      const plain = g.draw().map((l) => l.replace(/\x1b\[[\d;]+m/g, ""));
      expect(plain.length).toBeLessThanOrEqual(20);
      expect(plain.join("\n")).toContain("  P entry-29 ");
      expect(plain.join("\n")).toContain("Dashboards");
      expect(plain.join("\n")).toContain("Enter run");
      expect(plain.join("\n")).not.toContain("entry-0 ");
    });
  });
});
