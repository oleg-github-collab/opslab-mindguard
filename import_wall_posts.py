#!/usr/bin/env python3
"""
Import 6 wall posts from WALL_ALL_FEEDBACKS.json into production database via API
"""

import requests
import json

BASE_URL = "https://backend-production-e745.up.railway.app"
EMAIL = "work.olegkaminskyi@gmail.com"
CODE = "0000"

def main():
    print("=" * 70)
    print("IMPORTING WALL POSTS TO PRODUCTION")
    print("=" * 70)

    # Load extracted feedbacks
    with open('WALL_ALL_FEEDBACKS.json', 'r', encoding='utf-8') as f:
        feedbacks = json.load(f)

    print(f"\n📦 Loaded {len(feedbacks)} feedbacks from file")

    # Login
    print(f"\n🔐 Logging in as {EMAIL}...")
    session = requests.Session()
    login_resp = session.post(
        f"{BASE_URL}/auth/login",
        json={"email": EMAIL, "code": CODE},
        headers={"Content-Type": "application/json"}
    )

    if login_resp.status_code != 200:
        print(f"❌ Login failed: {login_resp.text}")
        return

    print("✅ Logged in successfully!")

    # Import each feedback
    imported = 0
    skipped = 0

    for i, fb in enumerate(feedbacks, 1):
        print(f"\n📝 [{i}/{len(feedbacks)}] Importing post from {fb['created_at'][:10]}...")
        print(f"   Sentiment: {fb['sentiment']}")
        print(f"   Summary: {fb['summary'][:50]}...")

        # Create wall post
        try:
            resp = session.post(
                f"{BASE_URL}/feedback/wall",
                json={"content": fb['summary']},
                headers={"Content-Type": "application/json"}
            )

            if resp.status_code in [200, 201]:
                imported += 1
                print(f"   ✅ Imported successfully!")
            else:
                skipped += 1
                print(f"   ⚠️ Skipped (status {resp.status_code}): {resp.text[:100]}")

        except Exception as e:
            skipped += 1
            print(f"   ❌ Error: {e}")

    # Summary
    print("\n" + "=" * 70)
    print("IMPORT COMPLETE")
    print("=" * 70)
    print(f"✅ Imported: {imported}")
    print(f"⚠️ Skipped: {skipped}")
    print(f"📊 Total: {len(feedbacks)}")
    print()

if __name__ == "__main__":
    main()
