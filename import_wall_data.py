#!/usr/bin/env python3
"""
Import extracted wall of tears data into the new platform
"""

import json
import os
import psycopg2
from datetime import datetime
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.backends import default_backend
import base64

# Database connection
DATABASE_URL = os.getenv("DATABASE_URL", "postgresql://postgres:ZYStivuqwNNVTPrNZkfAOWyqylnMDuXX@caboose.proxy.rlwy.net:37816/railway")

# Encryption key from environment (must match backend)
ENCRYPTION_KEY_HEX = os.getenv("ENCRYPTION_KEY", "")  # Will need to get this

def encrypt_content(plaintext: str, key_hex: str) -> str:
    """
    Encrypt content using AES-256-GCM to match Rust backend encryption
    """
    # Convert hex key to bytes
    key = bytes.fromhex(key_hex)

    # Generate random nonce (12 bytes for GCM)
    nonce = os.urandom(12)

    # Create cipher
    cipher = Cipher(
        algorithms.AES(key),
        modes.GCM(nonce),
        backend=default_backend()
    )
    encryptor = cipher.encryptor()

    # Encrypt
    ciphertext = encryptor.update(plaintext.encode('utf-8')) + encryptor.finalize()

    # Format: base64(nonce + ciphertext + tag)
    combined = nonce + ciphertext + encryptor.tag
    return base64.b64encode(combined).decode('utf-8')


def import_feedbacks():
    """Import the 6 extracted feedbacks into wall_posts table"""

    # Load extracted data
    with open('WALL_ALL_FEEDBACKS.json', 'r', encoding='utf-8') as f:
        feedbacks = json.load(f)

    print(f"📦 Loaded {len(feedbacks)} feedbacks from WALL_ALL_FEEDBACKS.json")

    # Connect to database
    conn = psycopg2.connect(DATABASE_URL)
    cur = conn.cursor()

    # Get encryption key from database
    cur.execute("SELECT key_hex FROM encryption_keys ORDER BY created_at DESC LIMIT 1")
    result = cur.fetchone()

    if not result:
        print("❌ No encryption key found in database!")
        return

    key_hex = result[0]
    print(f"🔐 Using encryption key: {key_hex[:16]}...")

    # Map work_aspect to PostCategory
    aspect_to_category = {
        'team': 'CELEBRATION',  # Team achievements -> celebration
        'management': 'COMPLAINT',  # Management issues -> complaint
        'workload': 'SUPPORT_NEEDED',  # Workload issues -> support needed
    }

    # Map sentiment to category (fallback)
    sentiment_to_category = {
        'positive': 'CELEBRATION',
        'negative': 'COMPLAINT',
        'mixed': 'SUGGESTION',
    }

    imported = 0

    for fb in feedbacks:
        # Reconstruct full content from summary + tags
        content = f"{fb['summary']}\n\nТеги: {', '.join(fb['tags'])}"

        # Encrypt content
        enc_content = encrypt_content(content, key_hex)

        # Determine category
        category = aspect_to_category.get(
            fb.get('work_aspect'),
            sentiment_to_category.get(fb.get('sentiment', 'mixed'), 'SUGGESTION')
        )

        # Get or create user_id (use Admin user for imported posts)
        # Since these are anonymous, we'll use the admin user ID
        cur.execute("SELECT id FROM users WHERE email = 'work.olegkaminskyi@gmail.com'")
        admin_user = cur.fetchone()

        if not admin_user:
            print(f"❌ Admin user not found!")
            continue

        user_id = admin_user[0]

        # Insert into wall_posts
        try:
            cur.execute("""
                INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
                VALUES (%s, %s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
            """, (
                fb['id'],
                user_id,
                enc_content.encode('utf-8'),
                category,
                True,  # These were AI-categorized in old system
                fb['created_at']
            ))

            imported += 1
            print(f"✅ Imported: {fb['id']} ({fb['sentiment']}, {category})")

        except Exception as e:
            print(f"❌ Failed to import {fb['id']}: {e}")

    # Commit
    conn.commit()
    cur.close()
    conn.close()

    print(f"\n🎉 Successfully imported {imported}/{len(feedbacks)} feedbacks!")


if __name__ == "__main__":
    import_feedbacks()
