export const kebab = (s: string) => s.replace(/_/g, "-").toLowerCase();
export const tagSlug = (tag: string) => tag.toLowerCase().replace(/\//g, "").replace(/\s+/g, "-");
export const singular = (s: string) => (s.endsWith("ies") ? s.slice(0, -3) + "y" : s.replace(/s$/, ""));

const isParam = (s: string) => s.startsWith("{");

const VERB: Record<string, string> = { GET: "get", POST: "create", PUT: "update", PATCH: "update", DELETE: "delete" };

export interface Name {
  resource: string;
  action: string;
  /** The literal before the collection (singular), used to split a collision. */
  parent?: string;
}

/** Does this path segment name the collection of the tag? */
function matches(seg: string, slug: string): boolean {
  const k = kebab(seg);
  return k === slug || k.endsWith(`-${slug}`) || slug.startsWith(k);
}

/** A spec tag that the client sees under another resource. `dashboard-v2` stays a hidden alias (index.ts). */
const RESOURCE_OF: Record<string, string> = { "dashboard-v2": "dashboard" };

export function deriveName(method: string, path: string, tag: string): Name {
  const n = nameOf(method, path, tag);
  return { ...n, resource: RESOURCE_OF[n.resource] ?? n.resource };
}

function nameOf(method: string, path: string, tag: string): Name {
  const slug = tagSlug(tag);
  let segs = path.replace(/^\/management\/v1\//, "").split("/");
  if (segs[0] === "applications" && segs[1] === "{application_id}" && segs.length > 2) segs = segs.slice(2);

  let ci = -1;
  for (let i = segs.length - 1; i >= 0; i--) {
    if (!isParam(segs[i]) && matches(segs[i], slug)) { ci = i; break; }
  }
  const literals = segs.filter((s) => !isParam(s));
  const endsWithParam = isParam(segs[segs.length - 1]);

  if (ci === -1) {
    // No collection literal: `colors_palette`, `web_flows/{id}/activate`.
    const last = literals[literals.length - 1];
    const prev = literals[literals.length - 2];
    return { resource: slug, action: prev ? `${kebab(last)}-${singular(kebab(prev))}` : kebab(last) };
  }

  const extra = segs.slice(ci + 1);
  const extraLits = extra.filter((s) => !isParam(s));
  const prevLit = segs.slice(0, ci).filter((s) => !isParam(s)).pop();
  const parent = prevLit ? singular(kebab(prevLit)) : undefined;
  let action: string;
  if (extraLits.length === 0) {
    if (endsWithParam) action = VERB[method];
    else if (method === "GET") action = ci > 0 && isParam(segs[ci - 1]) && singular(segs[ci]) === segs[ci] ? "get" : "list"; // singular sub-resource
    else action = method === "POST" ? "create" : "update";
  } else if (!endsWithParam) {
    action = kebab(extraLits.join("-"));
  } else {
    action = `${VERB[method]}-${singular(kebab(extraLits.join("-")))}`;
  }
  return { resource: slug, action, parent };
}
