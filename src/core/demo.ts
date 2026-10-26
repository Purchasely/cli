import { CliError, EXIT } from "./errors.js";
import type { HttpRequest, HttpResponse } from "./http.js";
import type { Deps } from "./types.js";

/**
 * Demo mode (`PURCHASELY_DEMO=1`): every request is answered here, from the sample data of one fictional app.
 * No key, no network, no limiter state, no usage line. The data is not real.
 */
export const demo = (d: Pick<Deps, "env">): boolean => d.env.PURCHASELY_DEMO === "1";

export const DEMO_APP = "app_demo";

export const demoWriteError = () =>
  new CliError(EXIT.REFUSED, "REFUSED", ["Demo mode (PURCHASELY_DEMO=1) never sends a write, so the CLI sent nothing. --dry-run and --plan still show the request."]);

const AT = "2026-01-12T09:30:00Z";
const UP = "2026-09-28T14:05:00Z";
const stamps = { created_at: AT, updated_at: UP };

const APPLICATION = {
  id: DEMO_APP,
  name: "Sample Meditation App (demo)",
  client_id: "cli_demo",
  default_language: "en",
  default_dashboard_currency: "USD",
  default_integration_currency: "USD",
  default_screen_id: "pres_demo_onboarding",
  redeem_presentation_id: null,
  icon_url: null,
  show_purchasely: true,
  has_mocked_data: true,
  client_webhook_url: null,
  s2s_notifications_endpoints: [],
  ios_platform: { bundle_id: "com.example.meditation.demo" },
  android_platform: { package_name: "com.example.meditation.demo" },
  stripe_platform: null,
  platforms_configured: { ios: true, android: true, stripe: false },
  ...stamps,
};

const SCREENS = [
  { id: "pres_demo_onboarding", name: "Onboarding paywall", vendor_id: "onboarding_paywall", status: "published", is_default: true, template_identifier: "full_screen", category_tag_ids: [], web_enabled: false, web_slug: null, web_slug_sandbox: null, ...stamps },
  { id: "pres_demo_upsell", name: "Home upsell", vendor_id: "home_upsell", status: "published", is_default: false, template_identifier: "bottom_sheet", category_tag_ids: [], web_enabled: false, web_slug: null, web_slug_sandbox: null, ...stamps },
  { id: "pres_demo_winback", name: "Winback offer", vendor_id: "winback_offer", status: "draft", is_default: false, template_identifier: "full_screen", category_tag_ids: [], web_enabled: false, web_slug: null, web_slug_sandbox: null, ...stamps },
];

const CAMPAIGN = {
  id: "cmp_demo_winback", name: "Winback after a cancellation", vendor_id: "winback", status: "running", position: 1,
  application_id: DEMO_APP, audience_id: "aud_demo_churned", ab_test_id: null, presentable_id: "pres_demo_winback",
  placement_ids: ["plc_demo_home"], category_tag_ids: [], config: {}, starts_at: "2026-09-01T00:00:00Z", ends_at: null, ...stamps,
};

const placement = (id: string, vendor_id: string, name: string, description: string, screen: string, extra: object = {}) => ({
  id, vendor_id, name, description, is_default: false, has_running_ab_test: false, default_presentable_id: screen,
  category_tag_ids: [], audiences: [], active_campaigns: [], ...stamps, ...extra,
});

const PLACEMENTS = [
  placement("plc_demo_onboarding", "onboarding", "Onboarding", "Shown at the end of the first launch", "pres_demo_onboarding", { is_default: true }),
  placement("plc_demo_home", "home", "Home banner", "The upgrade button of the home screen", "pres_demo_upsell", {
    audiences: [{ id: "aud_demo_churned", is_in_ab_test: false, priority: 1, presentable_id: "pres_demo_winback" }],
    active_campaigns: [{ id: CAMPAIGN.id, name: CAMPAIGN.name, audience_id: CAMPAIGN.audience_id, presentable_id: CAMPAIGN.presentable_id, starts_at: CAMPAIGN.starts_at, ends_at: null, position: 1, is_in_ab_test: false, ab_test_id: null, presentation_views: null, unique_views: null }],
  }),
  placement("plc_demo_settings", "settings", "Settings", "The subscription row of the settings", "pres_demo_upsell"),
];

const PRODUCT = { id: "prd_demo_premium", name: "Premium", vendor_id: "premium", description: "All the guided sessions", icon_url: null, default_screen_id: "pres_demo_onboarding", application_id: DEMO_APP, ...stamps };

const plan = (id: string, vendor_id: string, name: string, level: number, store_id: string, promo_offers: object[] = []) => ({
  id, name, vendor_id, product_type: "RENEWING_SUBSCRIPTION", level, period_type: "normal", commitment_billing_type: null, promo_offers,
  internal_notes: null, default_screen_id: null, product_id: PRODUCT.id, application_id: DEMO_APP,
  stores: { app_store: { platform_name: "ios", store_product_id: store_id }, play_store: { platform_name: "android", store_product_id: store_id }, stripe: [] },
  ...stamps,
});

const PLANS = [
  plan("pln_demo_monthly", "premium_monthly", "Premium monthly", 1, "com.example.meditation.demo.monthly"),
  plan("pln_demo_yearly", "premium_yearly", "Premium yearly", 2, "com.example.meditation.demo.yearly", [
    { id: "off_demo_trial", name: "7-day free trial", vendor_id: "yearly_trial", appstore_offer_id: null, playstore_offer_id: "trial-7d", stripe_trial_days: null, stripe_coupon_id: null, commitment_billing_type: null },
  ]),
];

const DAY_MS = 86_400_000;
const STEP: Record<string, number> = { DAY: 1, WEEK: 7, MONTH: 30 };
const ymd = (t: number) => new Date(t).toISOString().slice(0, 10);

/** The value on one day: a slow growth and a weekly wave, the same for a given date in every request. */
const wave = (day: number, base: number, growth: number, swing: number) => Math.max(0, Math.round(base + growth * (day - 20_000) + swing * Math.sin(day / 1.1)));

/** One bucket per DAY, WEEK or MONTH of `date_range[]`, labelled by its first day. */
function buckets(url: URL): { period: string; day: number; days: number }[] {
  const [from, to] = url.searchParams.getAll("date_range[]").map((s) => Math.floor(Date.parse(s.slice(0, 10)) / DAY_MS));
  const step = STEP[url.searchParams.get("periodicity") ?? "DAY"] ?? 1;
  if (!(from <= to)) return [];
  const out = [];
  for (let day = from; day <= to; day += step) out.push({ period: ymd(day * DAY_MS), day, days: Math.min(step, to - day + 1) });
  return out;
}

/** Viewers and conversions of one bucket: the sum of its days. */
const total = (b: { day: number; days: number }, base: number, swing: number) =>
  Array.from({ length: b.days }, (_, i) => wave(b.day + i, base, 0, swing)).reduce((a, v) => a + v, 0);

const DASHBOARDS: Record<string, (url: URL) => unknown> = {
  "dashboard/mrr_evolution_live": (url) => ({
    mrr_evolution_live: { data: { mrr: { values: { by_day: buckets(url).map((b) => ({ period: b.period, date_kinds: ["date_period"], value: wave(b.day, 24_000, 18, 150) })), by_month: [] }, variations: [] } } },
  }),
  "dashboard_v2/subscription_evolution": (url) => ({
    subscription_evolution: buckets(url).map((b) => ({ period: b.period, group_by: null, count: wave(b.day, 2_900, 2, 20) })),
  }),
  "dashboard_v2/conversion": (url) => {
    const bs = buckets(url);
    const viewers = bs.map((b) => ({ period: b.period, unique_viewers_count: total(b, 850, 120) }));
    return {
      conversion: {
        kpi: [{ unique_viewers_count: Math.round(viewers.reduce((a, v) => a + v.unique_viewers_count, 0) * 0.8) }],
        primary_metric_by_period: viewers,
        secondary_metric_by_period: bs.map((b) => ({ period: b.period, total_conversions_count: total(b, 30, 6) })),
      },
    };
  },
};

const ok = (body: unknown): HttpResponse => ({ status: 200, headers: new Headers({ "content-type": "application/json" }), text: JSON.stringify(body) });

/** A read of the sample data, shaped like the Management API. Anything else is a 404 that says demo mode has no data for it. */
export function demoResponse(req: HttpRequest, name: string): HttpResponse {
  const url = new URL(req.url);
  const path = url.pathname.replace(/^\/management\/v1(?=\/)/, "");
  const lists: Record<string, unknown> = { placements: { placements: PLACEMENTS }, screens: { screens: SCREENS }, products: { products: [PRODUCT] }, campaigns: { campaigns: [CAMPAIGN] } };
  const app = /^\/applications\/([^/]+)(?:\/(.*))?$/.exec(path);
  let body: unknown;
  if (req.method !== "GET") body = undefined;
  else if (path === "/applications") body = { mobile_applications: [APPLICATION] };
  else if (path.slice(1) in lists) body = lists[path.slice(1)];
  else if (app && decodeURIComponent(app[1]) === DEMO_APP) {
    const rest = app[2] ?? "";
    const one = /^(placements|screens|products|campaigns)\/([^/]+)$/.exec(rest);
    const find = <T extends { id: string; vendor_id: string }>(all: T[], key: string) => all.find((x) => x.id === key || x.vendor_id === key);
    if (rest === "") body = { mobile_application: APPLICATION };
    else if (rest in lists) body = lists[rest];
    else if (rest === "plans") body = url.searchParams.get("product_id") === PRODUCT.id ? { plans: PLANS } : undefined;
    else if (rest.startsWith("plans/")) body = ((p) => p && { plan: p })(find(PLANS, decodeURIComponent(rest.slice(6))));
    else if (rest in DASHBOARDS) body = DASHBOARDS[rest](url);
    else if (one) {
      const all = { placements: PLACEMENTS, screens: SCREENS, products: [PRODUCT], campaigns: [CAMPAIGN] }[one[1]]!;
      const hit = find(all as { id: string; vendor_id: string }[], decodeURIComponent(one[2]));
      body = hit && { [one[1].slice(0, -1)]: hit };
    }
  }
  if (body !== undefined) return ok(body);
  const message = `Demo mode (PURCHASELY_DEMO=1) has no sample data for ${req.method} ${req.pathTemplate}. The sample app is ${DEMO_APP}. Run \`${name} demo\` for the commands to try.`;
  return { status: 404, headers: new Headers({ "content-type": "application/json" }), text: JSON.stringify({ error: { code: "NOT_FOUND", messages: [message] } }) };
}
