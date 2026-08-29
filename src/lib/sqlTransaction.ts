export type SqlExecute = (sql: string, params?: unknown[]) => Promise<void>;

export async function runSqlTransaction<T>(
  execute: SqlExecute,
  work: () => Promise<T>,
): Promise<T> {
  await execute("BEGIN");
  try {
    const result = await work();
    await execute("COMMIT");
    return result;
  } catch (error) {
    try {
      await execute("ROLLBACK");
    } catch {
      // A failed rollback should not hide the original write error.
    }
    throw error;
  }
}
