// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Ticket 15 — n8n 2.39.0 import client.
 *
 * Posts an owner-authored n8n workflow JSON document to the
 * /api/v1/workflows/import/n8n endpoint and returns the server's
 * CompatibilityReport. Secret material is never read here: the server
 * fails-closed on credentials, secret-shaped keys, and large blobs.
 */

export type Classification =
  | "native_equivalent"
  | "delegated_compatible"
  | "preserved_opaque"
  | "adapted"
  | "unsupported"
  | "rejected_unsafe";

export interface ImportFinding {
  node_id: string | null;
  code: string;
  classification: Classification;
  message: string;
}

export interface ClassificationCounts {
  native_equivalent: number;
  delegated_compatible: number;
  preserved_opaque: number;
  adapted: number;
  unsupported: number;
  rejected: number;
}

export interface CompatibilityReport {
  source: {
    format: string;
    declared_version: string | null;
    exact_external_identity_preserved: boolean;
  };
  total_nodes: number;
  findings: ImportFinding[];
  classifications: ClassificationCounts;
  redactions: Array<{ node_id: string | null; path: string; reason: string }>;
  blocked: boolean;
  block_reason: string | null;
}

export interface ImportResponse {
  workflow_id: string;
  draft_version: number;
  node_count: number;
  compatibility_report: CompatibilityReport;
}

export class ImportRejectedError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
    this.name = "ImportRejectedError";
  }
}

export async function importN8nDocument(workflowId: string, document: unknown): Promise<ImportResponse> {
  const response = await fetch("/api/v1/workflows/import/n8n", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ workflow_id: workflowId, document }),
  });
  const body = await response.json().catch(() => ({})) as { code?: string; title?: string } & Partial<ImportResponse>;
  if (!response.ok) {
    throw new ImportRejectedError(
      response.status,
      body.code ?? "import_rejected",
      body.title ?? "Import rejected by the server",
    );
  }
  return body as ImportResponse;
}

/** Generate a short, url-safe workflow id from the provided hint. */
export function suggestedWorkflowId(hint: string): string {
  const slug = hint
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  const suffix = Math.random().toString(36).slice(2, 6);
  return `${slug || "imported-workflow"}-${suffix}`;
}
