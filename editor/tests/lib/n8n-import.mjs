// SPDX-License-Identifier: AGPL-3.0-or-later

// JS mirror of crates/workflowd/src/n8n_import.rs used for fast node:test
// coverage of the clean-room n8n v2 import rules. The Rust implementation is
// the authoritative server-side surface; this port keeps the same invariants
// so node:test can guard against sanitizer regressions without cargo.
//
// Supported first subset (n8n 2.39.0):
//   - Manual Trigger maps to canopy/manual-trigger/v1alpha1
//   - executeCommand is rejected fail-closed
//   - All other node types are Preserved Opaque with deterministic lock ids
//   - Secret-shaped keys / credential blocks are redacted
//   - Large base64-blob strings are redacted; oversized strings are rejected
//   - Connections are normalized; dangling edges become Adapted findings

const MAX_IMPORT_BYTES = 5 * 1024 * 1024;
const MAX_STRING_VALUE_BYTES = 8_192;
const MAX_NAME_CHARS = 200;

const SECRET_KEY_DENYLIST = [
  "password", "passwd", "token", "accesstoken", "access_token",
  "refreshtoken", "refresh_token", "apikey", "api_key", "secret",
  "clientsecret", "client_secret", "privatekey", "private_key",
  "authorization", "cookie", "set-cookie", "x-api-key", "x-auth-token",
];

const ALIAS_TABLE = new Map([
  ["n8n-nodes-base.manualTrigger@1", { kind: "native", ns: "canopy", name: "manual-trigger", ver: "v1alpha1" }],
  ["n8n-nodes-base.executeCommand@1", { kind: "rejected", reason: "executeCommand runs arbitrary shell commands; unsafe" }],
]);

export { MAX_IMPORT_BYTES };

export function import_n8n_v2(workflow_id, bytes) {
  if (bytes.length > MAX_IMPORT_BYTES) {
    throw new Error(`import exceeds ${MAX_IMPORT_BYTES} bytes (got ${bytes.length})`);
  }
  const text = bytes.toString("utf8");
  let src;
  try { src = JSON.parse(text); } catch (e) { throw new Error(`cannot parse n8n workflow JSON: ${e.message}`); }

  const nodes_in = Array.isArray(src.nodes) ? src.nodes : [];
  const connections_in = src.connections && typeof src.connections === "object" ? src.connections : {};
  let settings = src.settings && typeof src.settings === "object" && !Array.isArray(src.settings) ? { ...src.settings } : {};

  const report = {
    source: {
      format: "n8n.workflow-json/v2",
      declared_version: typeof src.versionId === "string" ? src.versionId : null,
      exact_external_identity_preserved: true,
    },
    total_nodes: nodes_in.length,
    findings: [],
    classifications: {
      native_equivalent: 0,
      delegated_compatible: 0,
      preserved_opaque: 0,
      adapted: 0,
      unsupported: 0,
      rejected: 0,
    },
    redactions: [],
    blocked: false,
    block_reason: null,
  };

  // Redact top-level credentials blocks on each node.
  for (const [i, node] of nodes_in.entries()) {
    if (node.credentials != null && (typeof node.credentials === "object")) {
      report.redactions.push({
        node_id: node.id ?? `n8n-import-${i}`,
        path: "credentials",
        reason: "credentials are redacted fail-closed; re-enter secrets through Canopy",
      });
      node.credentials = null;
    }
  }

  const nodes = [];
  const node_index = new Map();
  let blocked_reasons = [];

  for (let i = 0; i < nodes_in.length; i++) {
    const node = nodes_in[i];
    const name = typeof node.name === "string" ? node.name : `Imported node ${i + 1}`;
    if ([...name].length > MAX_NAME_CHARS) {
      throw new Error(`node #${i} name exceeds ${MAX_NAME_CHARS} characters`);
    }
    const external_id = typeof node.id === "string" && node.id.length ? node.id : `n8n-import-${i}`;
    const type_name = typeof node.type === "string" ? node.type : "";
    let type_version = 1;
    if (typeof node.typeVersion === "number") type_version = Math.trunc(node.typeVersion);

    let safe_params = deepClone(node.parameters ?? {});
    sanitize(safe_params, `nodes[${i}].parameters`, external_id, report);

    const [x, y] = parsePosition(node.position);
    const alias_key = `${type_name}@${type_version}`;
    const alias = ALIAS_TABLE.get(alias_key);

    let classification;
    let contract_lock;
    const compat_meta = {
      n8n_type: type_name,
      n8n_type_version: type_version,
      import_source: "n8n.workflow-json/v2",
    };

    if (!alias) {
      classification = "preserved_opaque";
      report.classifications.preserved_opaque += 1;
      contract_lock = opaqueLock(type_name, type_version);
      compat_meta.opaque_preserved = true;
    } else if (alias.kind === "native") {
      classification = "native_equivalent";
      report.classifications.native_equivalent += 1;
      contract_lock = {
        api_version: "canopy.node/v1alpha1",
        namespace: alias.ns,
        name: alias.name,
        version: alias.ver,
        digest: `mirror:${alias.ns}/${alias.name}/${alias.ver}`,
      };
      compat_meta.search_label = alias.name.replace(/-/g, " ");
    } else if (alias.kind === "rejected") {
      classification = "rejected_unsafe";
      report.classifications.rejected += 1;
      report.blocked = true;
      blocked_reasons.push(`node ${external_id} (${type_name}): ${alias.reason}`);
      contract_lock = opaqueLock(type_name, type_version);
    }

    report.findings.push({
      node_id: external_id,
      code: classification,
      classification,
      message: `${type_name} v${type_version} -> ${classification}`,
    });

    nodes.push({
      id: external_id,
      name: name.slice(0, MAX_NAME_CHARS),
      contract_lock,
      configuration: safe_params,
      layout: { x, y },
      annotation: "",
      compatibility_metadata: compat_meta,
    });
    node_index.set(external_id, i);
  }

  // settings sanitization
  sanitize(settings, "settings", null, report);

  if (report.blocked) {
    throw new Error(blocked_reasons.join("; "));
  }

  const connections = normalizeConnections(connections_in, node_index, report);

  return {
    draft: {
      workflow_id,
      name: typeof src.name === "string" ? src.name.slice(0, MAX_NAME_CHARS) : "Imported n8n workflow",
      draft_version: 1,
      nodes,
      connections,
      annotation: "",
      settings,
      compatibility_metadata: {
        import_source: "n8n.workflow-json/v2",
        n8n_version_target: "2.39.0",
      },
    },
    report,
  };
}

function sanitize(value, path, node_id, report) {
  if (typeof value === "string") {
    if (value.length > MAX_STRING_VALUE_BYTES) {
      throw new Error(`${path}: string value exceeds ${MAX_STRING_VALUE_BYTES} bytes`);
    }
    if (looksLikeBase64(value) && value.length > 1024) {
      report.redactions.push({ node_id, path, reason: "large base64-like blob redacted" });
      return "[REDACTED]";
    }
    return value;
  }
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) value[i] = sanitize(value[i], `${path}[${i}]`, node_id, report);
    return value;
  }
  if (value && typeof value === "object") {
    for (const k of Object.keys(value)) {
      const lc = k.toLowerCase();
      if (SECRET_KEY_DENYLIST.some(d => lc.includes(d))) {
        report.redactions.push({ node_id, path: `${path}.${k}`, reason: "secret-shaped key redacted fail-closed" });
        value[k] = "[REDACTED]";
        continue;
      }
      value[k] = sanitize(value[k], `${path}.${k}`, node_id, report);
    }
  }
  return value;
}

function looksLikeBase64(s) {
  if (s.length < 256) return false;
  const allowed = [...s].every(c => /[A-Za-z0-9+/=\r\n]/.test(c));
  return allowed && (s.replace(/[^A-Za-z0-9]/g, "").length * 4 / 3) >= s.length - 2;
}

function parsePosition(p) {
  if (Array.isArray(p) && p.length >= 2) return [Number(p[0]) || 0, Number(p[1]) || 0];
  if (p && typeof p === "object") return [Number(p.x) || 0, Number(p.y) || 0];
  return [0, 0];
}

function opaqueLock(type_name, version) {
  const name = `opaque-${type_name.replace(/[^A-Za-z0-9-]/g, "-").replace(/^-+|-+$/g, "").toLowerCase()}`;
  return {
    api_version: "canopy.node/v1alpha1",
    namespace: "n8n-import",
    name,
    version: `v${version}`,
    digest: `mirror:opaque/${type_name}/v${version}`,
  };
}

function normalizeConnections(c, index, report) {
  const out = [];
  const seen = new Set();
  for (const [src_name, ports] of Object.entries(c)) {
    if (!index.has(src_name)) continue;
    if (!ports || typeof ports !== "object") continue;
    for (const [port_name, out_lists] of Object.entries(ports)) {
      if (!Array.isArray(out_lists)) continue;
      out_lists.forEach((targets, out_idx) => {
        if (!Array.isArray(targets)) return;
        targets.forEach((tgt, in_idx) => {
          if (!tgt || typeof tgt !== "object") return;
          const tgt_name = tgt.node;
          const tgt_port = typeof tgt.type === "string" ? tgt.type : "main";
          if (!index.has(tgt_name)) {
            report.findings.push({
              node_id: src_name,
              code: "dangling_connection",
              classification: "adapted",
              message: `connection to unknown target '${tgt_name}' dropped`,
            });
            report.classifications.adapted += 1;
            return;
          }
          const id = `imp-${src_name}-${port_name}-${out_idx}-${tgt_name}-${tgt_port}-${in_idx}`;
          if (seen.has(id)) return;
          seen.add(id);
          out.push({
            id,
            source: { node_id: src_name, port_id: port_name },
            target: { node_id: tgt_name, port_id: tgt_port },
          });
        });
      });
    }
  }
  return out;
}

function deepClone(v) {
  if (v == null || typeof v !== "object") return v;
  if (Array.isArray(v)) return v.map(deepClone);
  const out = {};
  for (const k of Object.keys(v)) out[k] = deepClone(v[k]);
  return out;
}
