import assert from "node:assert/strict";
import test from "node:test";
import {
  extractSelectColumns,
  mapProxyRowValues,
  mapSqliteProxyResult,
} from "../src/lib/sqliteProxy.ts";

const NOTES_SQL =
  'select "id", "content", "color", "pinned", "sort_order", "text_height", "created_at", "updated_at" from "notes" order by "notes"."pinned" desc';

test("extracts quoted select columns in query order", () => {
  assert.deepEqual(extractSelectColumns(NOTES_SQL), [
    "id",
    "content",
    "color",
    "pinned",
    "sort_order",
    "text_height",
    "created_at",
    "updated_at",
  ]);
});

test("extracts aliased and table-qualified columns", () => {
  assert.deepEqual(
    extractSelectColumns('select "settings"."value" as "value" from "settings" where "key" = ?'),
    ["value"],
  );
  assert.deepEqual(extractSelectColumns('select "notes"."id", "notes"."content" from "notes"'), [
    "id",
    "content",
  ]);
});

test("maps rows by select order even when object keys are alphabetical", () => {
  const row = {
    color: "#fff9db",
    content: "hello",
    created_at: 10,
    id: "note-1",
    pinned: 1,
    sort_order: -20,
    text_height: null,
    updated_at: 30,
  };

  assert.deepEqual(mapProxyRowValues(row, NOTES_SQL), [
    "note-1",
    "hello",
    "#fff9db",
    1,
    -20,
    null,
    10,
    30,
  ]);
});

test("reads unqualified column names from prefixed select lists", () => {
  assert.equal(
    mapProxyRowValues(
      { value: '{"language":"zh"}' },
      'select "settings"."value" from "settings"',
    )[0],
    '{"language":"zh"}',
  );
});

test("returns the first mapped row for get queries", () => {
  const result = mapSqliteProxyResult(
    'select "id", "content" from "notes"',
    [{ content: "hello", id: "note-1" }],
    "get",
  );

  assert.deepEqual(result.rows, ["note-1", "hello"]);
});

test("keeps all mapped rows for all queries", () => {
  const result = mapSqliteProxyResult(
    'select "id", "content" from "notes"',
    [
      { content: "one", id: "a" },
      { content: "two", id: "b" },
    ],
    "all",
  );

  assert.deepEqual(result.rows, [
    ["a", "one"],
    ["b", "two"],
  ]);
});
