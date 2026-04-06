/**
 * Minimal type declarations for sql.js (pure JavaScript SQLite).
 * Only the APIs used by credential-manager.ts are typed here.
 */
declare module 'sql.js' {
  interface SqlJsDatabase {
    run(sql: string, params?: unknown[]): void;
    exec(sql: string, params?: unknown[]): Array<{ columns: string[]; values: unknown[][] }>;
    export(): Uint8Array;
    close(): void;
  }

  interface SqlJsStatic {
    Database: new (data?: ArrayLike<number> | Buffer | null) => SqlJsDatabase;
  }

  interface InitSqlJsOptions {
    locateFile?: (file: string) => string;
  }

  function initSqlJs(options?: InitSqlJsOptions): Promise<SqlJsStatic>;

  export default initSqlJs;
  export type { SqlJsDatabase, SqlJsStatic };
}
