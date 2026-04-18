// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
import { Pool } from "pg";

export const pool = new Pool({ connectionString: process.env.DATABASE_URL });

/**
 * Scope a database connection to a tenant before running queries.
 * Every subsequent query automatically applies RLS.
 */
export async function withTenant<T>(
  tenantId: string,
  fn: (client: import("pg").PoolClient) => Promise<T>,
): Promise<T> {
  const client = await pool.connect();
  try {
    await client.query(`SET LOCAL app.tenant_id = $1`, [tenantId]);
    return await fn(client);
  } finally {
    client.release();
  }
}
