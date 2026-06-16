import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const notesTable = sqliteTable("notes", {
  id: text("id").primaryKey().notNull(),
  content: text("content").notNull(),
  color: text("color").notNull(),
  pinned: integer("pinned").notNull().default(0),
  sortOrder: integer("sort_order").notNull().default(0),
  textHeight: integer("text_height"),
  createdAt: integer("created_at").notNull(),
  updatedAt: integer("updated_at").notNull(),
});

export const attachmentsTable = sqliteTable("attachments", {
  id: text("id").primaryKey().notNull(),
  noteId: text("note_id").notNull(),
  kind: text("kind", { enum: ["image", "file"] })
    .notNull()
    .default("image"),
  source: text("source", { enum: ["data", "url", "path"] }).notNull(),
  value: text("value").notNull(),
  name: text("name"),
  createdAt: integer("created_at").notNull(),
});

export const settingsTable = sqliteTable("settings", {
  key: text("key").primaryKey().notNull(),
  value: text("value").notNull(),
});

export const schema = {
  attachmentsTable,
  notesTable,
  settingsTable,
};
