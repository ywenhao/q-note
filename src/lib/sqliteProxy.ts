export type SqliteProxyMethod = "all" | "get" | "values";

export const SQLITE_TABLE_COLUMNS: Record<string, string[]> = {
  notes: [
    "id",
    "content",
    "color",
    "pinned",
    "sort_order",
    "text_height",
    "created_at",
    "updated_at",
  ],
  attachments: ["id", "note_id", "kind", "source", "value", "name", "created_at"],
  settings: ["key", "value"],
};

const SELECT_FROM_RE = /^select\s+([\s\S]+?)\s+from\s+/i;
const SELECT_STAR_FROM_RE = /^select\s+\*\s+from\s+"?([A-Za-z_][\w]*)"?/i;

export function extractSelectColumns(sql: string): string[] | null {
  const trimmed = sql.trim();
  const star = trimmed.match(SELECT_STAR_FROM_RE);
  if (star) {
    return SQLITE_TABLE_COLUMNS[star[1].toLowerCase()] ?? null;
  }

  const match = trimmed.match(SELECT_FROM_RE);
  if (!match) {
    return null;
  }

  const list = match[1].trim();
  if (!list || list === "*" || /^\*\s*,/.test(list)) {
    return null;
  }

  return splitSelectList(list).map(normalizeSelectColumn);
}

export function readProxyRowValue(row: Record<string, unknown>, column: string): unknown {
  if (Object.prototype.hasOwnProperty.call(row, column)) {
    return row[column];
  }

  const unquoted = stripQuotes(column);
  if (unquoted !== column && Object.prototype.hasOwnProperty.call(row, unquoted)) {
    return row[unquoted];
  }

  const last = unquoted.split(".").pop() ?? unquoted;
  if (last !== unquoted && Object.prototype.hasOwnProperty.call(row, last)) {
    return row[last];
  }

  return undefined;
}

export function mapProxyRowValues(row: Record<string, unknown>, sql: string): unknown[] {
  const columns = extractSelectColumns(sql) ?? inferColumnsFromRow(row);
  return columns.map((column) => readProxyRowValue(row, column));
}

function inferColumnsFromRow(row: Record<string, unknown>): string[] {
  const known = Object.values(SQLITE_TABLE_COLUMNS)
    .flat()
    .filter((column, index, all) => all.indexOf(column) === index && column in row);
  if (known.length > 0) {
    return known;
  }
  return Object.keys(row);
}

export function mapSqliteProxyResult(
  sql: string,
  rows: Record<string, unknown>[],
  method: SqliteProxyMethod,
): { rows: unknown[] } {
  const values = rows.map((row) => mapProxyRowValues(row, sql));
  return { rows: (method === "get" ? values[0] : values) as unknown[] };
}

function stripQuotes(value: string): string {
  return value.replace(/"/g, "");
}

function normalizeSelectColumn(raw: string): string {
  const trimmed = raw.trim();
  const alias = trimmed.match(/\bas\s+("([^"]+)"|([A-Za-z_][\w$]*))$/i);
  if (alias) {
    return alias[2] ?? alias[3] ?? trimmed;
  }

  const last = trimmed.split(".").pop()?.trim() ?? trimmed;
  return stripQuotes(last);
}

function splitSelectList(list: string): string[] {
  const columns: string[] = [];
  let current = "";
  let quote: '"' | null = null;
  let depth = 0;

  for (const char of list) {
    if (quote) {
      current += char;
      if (char === quote) {
        quote = null;
      }
      continue;
    }

    if (char === '"') {
      quote = '"';
      current += char;
      continue;
    }

    if (char === "(") {
      depth += 1;
      current += char;
      continue;
    }

    if (char === ")") {
      depth = Math.max(0, depth - 1);
      current += char;
      continue;
    }

    if (char === "," && depth === 0) {
      if (current.trim()) {
        columns.push(current.trim());
      }
      current = "";
      continue;
    }

    current += char;
  }

  if (current.trim()) {
    columns.push(current.trim());
  }

  return columns;
}
