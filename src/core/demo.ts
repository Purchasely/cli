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
function buckets(url: URL, def = "DAY"): { period: string; day: number; days: number }[] {
  const [from, to] = url.searchParams.getAll("date_range[]").map((s) => Math.floor(Date.parse(s.slice(0, 10)) / DAY_MS));
  const step = STEP[url.searchParams.get("periodicity") ?? def] ?? 1;
  if (!(from <= to)) return [];
  const out = [];
  for (let day = from; day <= to; day += step) out.push({ period: ymd(day * DAY_MS), day, days: Math.min(step, to - day + 1) });
  return out;
}

/** Viewers and conversions of one bucket: the sum of its days. */
const total = (b: { day: number; days: number }, base: number, swing: number) =>
  Array.from({ length: b.days }, (_, i) => wave(b.day + i, base, 0, swing)).reduce((a, v) => a + v, 0);


/** Active subscriptions of one day (about 4,400 today). */
const active = (day: number) => wave(day, 2_900, 2, 20);

/** `n` split by shares, rounded. */
const split = (n: number, shares: Record<string, number>) => Object.fromEntries(Object.entries(shares).map(([k, s]) => [k, Math.round(n * s)]));
const STORES = { google: 0.4, apple: 0.5, stripe: 0.1 };
const COUNTRIES = { "United States": 0.45, France: 0.25, "United Kingdom": 0.2, Germany: 0.1 };
const PLAN_NAMES = { "Premium monthly": 0.6, "Premium yearly": 0.4 };

/** One row per bucket, `{ period, ...fn(bucket) }`. The API default periodicity is MONTH. */
const series = (url: URL, fn: (b: ReturnType<typeof buckets>[number]) => object, def = "MONTH") => buckets(url, def).map((b) => ({ period: b.period, ...fn(b) }));
/** The number of days of the range. */
const days = (url: URL) => buckets(url, "DAY").length;

/** Revenue by store, country and plan: the shape of the subscription and one-time revenue dashboards. */
const revenue = (url: URL, base: number, plans: Record<string, number>) => ({
  data: {
    revenue: { values: series(url, (b) => split(total(b, base, 150), STORES)) },
    revenue_by_country: { values: series(url, (b) => split(total(b, base, 150), COUNTRIES)) },
    revenue_by_plan: { values: series(url, (b) => split(total(b, base, 150), plans)) },
  },
});

const color = (k: string) => (/not_renewing|grace|retry/.test(k) ? "orange" : /renewing|paused|subscribed|renewed/.test(k) ? "green" : "red");
/** One card of the header dashboards. */
const card = (category: string, vals: Record<string, number>) => {
  const sum = Object.values(vals).reduce((a, v) => a + v, 0);
  return {
    title: category[0].toUpperCase() + category.slice(1).replace(/_/g, " "), category, total: `${sum} - 100.0%`, downloadable: false, with_link: false,
    data: Object.entries(vals).map(([k, val]) => ({ title_key: k, subcategory: k, val, percentage: sum ? Math.round((val / sum) * 1000) / 10 : 0, color: color(k), with_link: false, downloadable: false })),
  };
};

/** Cohort rows of the retention matrix: one cohort per calendar month of the range. */
function retention(url: URL) {
  const [from, to] = url.searchParams.getAll("date_range[]").map((s) => new Date(s.slice(0, 10) + "T00:00:00Z"));
  const months: string[] = [];
  if (from <= to) for (let d = new Date(Date.UTC(from.getUTCFullYear(), from.getUTCMonth(), 1)); d <= to; d = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + 1, 1))) months.push(d.toISOString().slice(0, 10));
  const size = 140;
  const rows = months.flatMap((cohort, c) => months.slice(c).map((m, k) => ({
    cohort_date: cohort, period_date: m, period_name: `MONTH ${k}`, is_partial: c + k === months.length - 1,
    retained_count: Math.round(size * 0.82 ** k), retained_ratio: Math.round(0.82 ** k * 1000) / 1000,
  })));
  const revenue_per = 9.5;
  const totals = months.map((cohort) => ({ cohort_date: cohort, total_count: size, total_revenue_in_dashboard_currency: Math.round(size * 2.4 * revenue_per), revenue_per_subscription_in_dashboard_currency: Math.round(2.4 * revenue_per * 100) / 100 }));
  const periods = months.map((m, k) => {
    const cohorts = months.length - k;
    return { period_name: `MONTH ${k}`, period_date: m, retained_count: Math.round(size * 0.82 ** k) * cohorts, total_count: size * cohorts, retained_ratio: Math.round(0.82 ** k * 1000) / 1000 };
  });
  return { rows, totals, grand_total: { total_count: size * months.length, total_revenue_in_dashboard_currency: Math.round(size * 2.4 * revenue_per) * months.length, revenue_per_subscription_in_dashboard_currency: Math.round(2.4 * revenue_per * 100) / 100, periods } };
}

/** A flow of subscriptions: `base` a day, rows without a group (the demo ignores group_by). */
const flow = (key: string, count: string, base: number) => (url: URL) => ({
  [key]: series(url, (b) => ({ group_by: null, group_by_public_id: null, group_by_name: null, [count]: total(b, base, 5) })),
});

/** The last `n` calendar months up to this one, oldest first. */
const lastMonths = (n: number) => Array.from({ length: n }, (_, i) => { const d = new Date(); d.setUTCDate(1); d.setUTCMonth(d.getUTCMonth() - (n - 1 - i)); return d.toISOString().slice(0, 7); });

const FUNNEL_GROUPS: Record<string, string[]> = {
  platform: ["IOS", "ANDROID", "STRIPE"], country: ["US", "FR", "GB"], placement: ["plc_demo_onboarding", "plc_demo_home", "plc_demo_settings"],
  plan: ["pln_demo_monthly", "pln_demo_yearly", "pln_demo_monthly"], screen: ["pres_demo_onboarding", "pres_demo_upsell", "pres_demo_winback"],
};

const DASHBOARDS: Record<string, (url: URL) => unknown> = {
  "dashboard/mrr_evolution_live": (url) => ({
    mrr_evolution_live: { data: { mrr: { values: { by_day: buckets(url).map((b) => ({ period: b.period, date_kinds: ["date_period"], value: wave(b.day, 24_000, 18, 150) })), by_month: [] }, variations: [] } } },
  }),
  "dashboard_v2/subscription_evolution": (url) => ({
    subscription_evolution: buckets(url).map((b) => ({ period: b.period, group_by: null, count: active(b.day) })),
  }),
  "dashboard/composition_live": () => ({
    composition_live: { data: {
      composition_plan: { values: [{ name: "Premium monthly", value: 2_600 }, { name: "Premium yearly", value: 1_800 }] },
      composition_platform: { values: [{ name: "ios", value: 2_200 }, { name: "android", value: 1_900 }, { name: "stripe", value: 300 }] },
      composition_product: { values: [{ name: "Premium", value: 4_400 }] },
      subscription_state: { active: 3_700, active_intro: 60, active_trial: 240, canceled_intro: 10, canceled_trial: 90, churners: 300 },
    } },
  }),
  "dashboard/events": (url) => ({
    events: { data: Object.fromEntries(series(url, (b) => ({ PRESENTATION_VIEWED: total(b, 850, 120), SUBSCRIPTION_STARTED: total(b, 30, 6) })).map((r) => [r.period, r])) },
  }),
  "dashboard/flow_paths": () => ({
    flow_paths: { data: { steps: [{ screen_id: "pres_demo_onboarding", views: 700, next: [{ screen_id: "pres_demo_upsell", count: 260 }] }, { screen_id: "pres_demo_upsell", views: 260, next: [] }] } },
  }),
  "dashboard/funnel": (url) => {
    const n = days(url);
    const groups = FUNNEL_GROUPS[url.searchParams.get("dimension") ?? "platform"] ?? FUNNEL_GROUPS.platform;
    return { funnel: { data: groups.map((name, i) => {
      const s = [0.5, 0.4, 0.1][i];
      const c = (v: number) => Math.round(v * n * s);
      return { name, results: { subscription_started_count: c(25), subscription_terminated_count: c(3), trial_started_count: c(12), trial_converted_count: c(9), trial_not_converted_count: c(3), first_renewals_count: c(20) } };
    }) } };
  },
  "dashboard/header_live": (url) => {
    const cards: Record<string, ReturnType<typeof card>> = {
      active_subscriptions_statuses: card("active_subscriptions_statuses", { active_renewing: 3_500, active_not_renewing: 420, active_grace_period: 60, trial_renewing: 180, trial_not_renewing: 70, trial_grace_period: 5, intro_period_renewing: 50, intro_period_not_renewing: 10, intro_period_grace_period: 0, promo_code_renewing: 80, promo_code_not_renewing: 15, promo_code_grace_period: 0, promotional_offer_renewing: 10, promotional_offer_not_renewing: 0, promotional_offer_grace_period: 0 }),
      inactive_subscriptions_causes: card("inactive_subscriptions_causes", { terminated: 1_900, refunded: 40, unpaid: 25, paused: 12, billing_retry: 90, fraud_attemps_blocked: 0 }),
    };
    const only = url.searchParams.get("category");
    return { header_live: { data: only ? [cards[only]].filter(Boolean) : Object.values(cards) } };
  },
  "dashboard/header_today": () => ({
    header_today: { data: [{ ...card("today", { subscribed_with_free_trial: 9, subscribed_with_intro: 2, subscribed_with_promo_code: 1, subscribed_with_promotional_offer: 0, subscribed_paying: 18, renewed: 95, entered_grace_period: 3, churned: 14 }), title: "Today" }] },
  }),
  "dashboard/one_time_purchases_revenues": (url) => ({ one_time_purchases_revenues: revenue(url, 40, { "200 coins": 0.7, "1000 coins": 0.3 }) }),
  "dashboard/one_time_purchases_stats": (url) => ({
    one_time_purchases_stats: { data: {
      otp_by_store: { values: series(url, (b) => split(total(b, 3, 1), STORES)) },
      otp_by_plan: { values: series(url, (b) => split(total(b, 3, 1), { "200 coins": 0.7, "1000 coins": 0.3 })) },
      otp_refunds: { values: series(url, () => ({ percent: 1.5 })) },
    } },
  }),
  "dashboard/subscriptions_revenues": (url) => ({ subscriptions_revenues: revenue(url, 1_200, PLAN_NAMES) }),
  "dashboard/subscriptions_stats": (url) => ({
    subscriptions_stats: { data: {
      active: { values: series(url, (b) => ({ value: active(b.day) })) },
      new_churn: { values: series(url, (b) => { const n = total(b, 30, 6); const churn = total(b, 24, 5); return { new: n, churn, sum: n - churn }; }), variations: [] },
      refund: { values: series(url, (b) => { const count = Math.round(total(b, 30, 6)); const refund_count = Math.round(count * 0.015); return { refund_count, subscription_count: count, percent: Math.round((refund_count / count) * 1000) / 10 }; }) },
    } },
  }),
  "dashboard/trials_live": (url) => {
    const trials = (b: ReturnType<typeof buckets>[number], p: string) => ({
      [`${p}_converted`]: total(b, 9, 3), [`${p}_not_converted`]: total(b, 3, 2), [`${p}_ongoing_auto_renewing`]: total(b, 6, 2),
      [`${p}_ongoing_billing_retry`]: 0, [`${p}_ongoing_grace_period`]: 0, [`${p}_ongoing_non_auto_renewing`]: total(b, 1, 1),
    });
    const balance = (b: ReturnType<typeof buckets>[number], p: string) => ({
      [`${p}_started`]: total(b, 12, 3), [`${p}_started_auto_renewing`]: total(b, 11, 3), [`${p}_started_non_auto_renewing`]: total(b, 1, 1),
      [`${p}_converted`]: total(b, 9, 3), [`${p}_not_converted`]: total(b, 3, 2), balance: total(b, 0, 2),
    });
    return { trials_live: { data: {
      trials: { values: series(url, (b) => trials(b, "trials")) },
      intro_offers: { values: series(url, (b) => trials(b, "intro_offers")) },
      trials_balance: { values: series(url, (b) => balance(b, "trials")) },
      intro_offers_balance: { values: series(url, (b) => balance(b, "intro_offers")) },
    } } };
  },
  "dashboard_v2/mau_reports": () => {
    const months = lastMonths(10);
    const users = months.map((period, i) => ({ period, value: Math.round(15_000 + 400 * i + 800 * Math.sin(i)) }));
    return { mau_reports: { data: { app_starts: users.map((u) => ({ period: u.period, value: u.value * 8 })), unique_users: users } } };
  },
  "dashboard_v2/paid_subscription_movements": (url) => ({
    paid_subscription_movements: series(url, (b) => {
      const total_activations_count = total(b, 28, 5);
      const total_churns_count = total(b, 22, 5);
      return { group_by: null, group_by_public_id: null, group_by_name: null, total_activations_count, total_churns_count, net_movement_count: total_activations_count - total_churns_count };
    }),
  }),
  "dashboard_v2/paid_subscription_retention": (url) => ({ paid_subscription_retention: retention(url) }),
  "dashboard_v2/paywall_latency": (url) => {
    const n = Math.min(90, Math.max(7, Number(url.searchParams.get("period")) || 30));
    const dates = Array.from({ length: n }, (_, i) => ymd(Date.now() - (n - 1 - i) * DAY_MS));
    const at = (base: number) => dates.map((d) => wave(Math.floor(Date.parse(d) / DAY_MS), base, 0, base / 10));
    return { paywall_latency: { data: { dates, p50: at(120), p95: at(340), p99: at(780), request_count: at(2_400), countries: ["US", "FR", "GB"] } } };
  },
  "dashboard_v2/sdk_evolution": flow("sdk_evolution", "app_starts_count", 2_500),
  "dashboard_v2/sdk_versions": () => ({
    sdk_versions: [{ sdk_version: "6.2.0", events_count: 84_000 }, { sdk_version: "6.1.0", events_count: 3_500 }, { sdk_version: "6.3.0", events_count: 900 }, { sdk_version: "6.0.0", events_count: 20 }],
  }),
  "dashboard_v2/subscription_activations": flow("subscription_activations", "total_activations_count", 30),
  "dashboard_v2/subscription_reactivations": flow("subscription_reactivations", "total_reactivations_count", 4),
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
