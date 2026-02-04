#!/usr/bin/env python3
import psycopg2
import os

DATABASE_URL = "postgresql://postgres:ZYStivuqwNNVTPrNZkfAOWyqylnMDuXX@caboose.proxy.rlwy.net:37816/railway"

def run_migration():
    print("Connecting to database...")
    conn = psycopg2.connect(DATABASE_URL)
    cur = conn.cursor()

    print("Running migration 19...")

    # Read migration file
    with open('migrations/19_checkin_reminder_followups.sql', 'r') as f:
        sql = f.read()

    # Execute migration
    cur.execute(sql)
    conn.commit()

    print("Migration 19 completed successfully!")
    print("Verifying column was added...")

    # Verify
    cur.execute("""
        SELECT column_name, data_type
        FROM information_schema.columns
        WHERE table_name = 'user_preferences'
        AND column_name = 'last_reminder_stage'
    """)

    result = cur.fetchone()
    if result:
        print(f"✓ Column added: {result[0]} ({result[1]})")
    else:
        print("✗ Column not found!")

    cur.close()
    conn.close()

if __name__ == "__main__":
    run_migration()
