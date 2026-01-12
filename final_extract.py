#!/usr/bin/env python3
"""
FINAL DATA EXTRACTION - Uses discovered /api/ endpoints with Bearer tokens
"""

import requests
import json

EMAIL = "work.olegkaminskyi@gmail.com"
PASSWORD = "QwertY24$"

def extract_wall_of_tears():
    print("=" * 80)
    print("🧱 EXTRACTING WALL OF TEARS DATA")
    print("=" * 80)

    base = "https://opslab-feedback-production.up.railway.app/api"
    s = requests.Session()

    # Login
    print(f"\n🔐 Logging in as {EMAIL}...")
    login = s.post(f"{base}/auth/login", json={"email": EMAIL, "password": PASSWORD})

    if login.status_code != 200:
        print(f"❌ Login failed: {login.text}")
        return None

    data = login.json()
    token = data['access_token']
    user = data['user']

    print(f"✅ Logged in as {user['full_name']} ({user['role']})")
    print(f"   User ID: {user['id']}")

    # Set authorization header
    s.headers.update({"Authorization": f"Bearer {token}"})

    # Get stats
    print(f"\n📊 Fetching stats...")
    stats_resp = s.get(f"{base}/stats")
    print(f"   Status: {stats_resp.status_code}")
    print(f"   Response length: {len(stats_resp.text)}")

    if stats_resp.status_code == 200 and stats_resp.text:
        try:
            stats = stats_resp.json()
            print(f"   ✅ Keys: {list(stats.keys())}")

            with open("wall_stats.json", "w", encoding="utf-8") as f:
                json.dump(stats, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_stats.json")
        except:
            print(f"   ⚠️ Empty or invalid JSON response")
    else:
        print(f"   ⚠️ Empty response")

    # Get available months
    print(f"\n📅 Fetching available months...")
    months_resp = s.get(f"{base}/stats/available-months")
    print(f"   Status: {months_resp.status_code}")
    print(f"   Response length: {len(months_resp.text)}")

    if months_resp.status_code == 200 and months_resp.text:
        try:
            months = months_resp.json()
            print(f"   ✅ Got {len(months)} months: {months}")

            with open("wall_months.json", "w", encoding="utf-8") as f:
                json.dump(months, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_months.json")
        except:
            print(f"   ⚠️ Invalid JSON")
            months = []
    else:
        print(f"   ⚠️ Empty response")
        months = []

    # Get stats for each month
    all_monthly = []
    if months:
        for month in months:
            print(f"\n   📦 Fetching {month}...")
            month_resp = s.get(f"{base}/stats", params={"month": month})

            if month_resp.status_code == 200 and month_resp.text:
                try:
                    month_data = month_resp.json()
                    all_monthly.append({"month": month, "data": month_data})
                    print(f"      ✅ Success")
                except:
                    print(f"      ⚠️ Invalid JSON")
            else:
                print(f"      ❌ Failed: {month_resp.status_code}")

    if all_monthly:
        with open("wall_all_monthly.json", "w", encoding="utf-8") as f:
            json.dump(all_monthly, f, indent=2, ensure_ascii=False, default=str)
        print(f"\n   💾 Saved {len(all_monthly)} months to wall_all_monthly.json")

    # Get feedbacks/posts
    print(f"\n💬 Fetching feedbacks...")
    feedbacks_resp = s.get(f"{base}/feedbacks")
    print(f"   Status: {feedbacks_resp.status_code}")
    print(f"   Response length: {len(feedbacks_resp.text)}")

    if feedbacks_resp.status_code == 200 and feedbacks_resp.text:
        try:
            feedbacks = feedbacks_resp.json()
            count = len(feedbacks) if isinstance(feedbacks, list) else "?"
            print(f"   ✅ Got {count} feedbacks")

            with open("wall_feedbacks.json", "w", encoding="utf-8") as f:
                json.dump(feedbacks, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_feedbacks.json")
        except:
            print(f"   ⚠️ Invalid JSON")
    else:
        print(f"   ⚠️ Empty or failed response")

    # Try other endpoints
    endpoints = ["/feedback", "/api/wall", "/wall"]
    for ep in endpoints:
        print(f"\n🔍 Trying {ep}...")
        resp = s.get(f"https://opslab-feedback-production.up.railway.app{ep}")
        print(f"   Status: {resp.status_code}")

        if resp.status_code == 200:
            try:
                data = resp.json()
                filename = f"wall_{ep.replace('/', '_').strip('_')}.json"
                with open(filename, "w", encoding="utf-8") as f:
                    json.dump(data, f, indent=2, ensure_ascii=False, default=str)
                print(f"   ✅ Saved to {filename}")
            except:
                pass

    return True


def extract_teampulse():
    print("\n" + "=" * 80)
    print("💓 EXTRACTING TEAMPULSE DATA")
    print("=" * 80)

    base = "https://teampulse-mindguard-production.up.railway.app/api"
    s = requests.Session()

    # Login
    print(f"\n🔐 Logging in as {EMAIL}...")
    login = s.post(f"{base}/auth/login", json={"email": EMAIL, "password": PASSWORD})

    if login.status_code != 200:
        print(f"❌ Login failed ({login.status_code}): {login.text[:200]}")
        return None

    data = login.json()
    token = data.get('access_token')

    if not token:
        print(f"❌ No access_token in response: {list(data.keys())}")
        return None

    user = data.get('user', {})
    print(f"✅ Logged in as {user.get('full_name', '?')} ({user.get('role', '?')})")

    # Set authorization
    s.headers.update({"Authorization": f"Bearer {token}"})

    # Try different endpoints
    endpoints = [
        "/stats",
        "/stats/available-months",
        "/feedbacks",
        "/dashboard",
        "/users",
        "/health",
    ]

    for ep in endpoints:
        print(f"\n📊 Fetching {ep}...")
        resp = s.get(f"{base}{ep}")
        print(f"   Status: {resp.status_code}")

        if resp.status_code == 200:
            try:
                data = resp.json()
                filename = f"teampulse_{ep.replace('/', '_').strip('_')}.json"
                with open(filename, "w", encoding="utf-8") as f:
                    json.dump(data, f, indent=2, ensure_ascii=False, default=str)
                print(f"   ✅ Saved to {filename}")

                # If it's months, fetch each month's data
                if ep == "/stats/available-months" and isinstance(data, list):
                    all_monthly = []
                    for month in data:
                        print(f"      📦 Fetching {month}...")
                        month_resp = s.get(f"{base}/stats", params={"month": month})

                        if month_resp.status_code == 200:
                            month_data = month_resp.json()
                            all_monthly.append({"month": month, "data": month_data})
                            print(f"         ✅ Success")

                    if all_monthly:
                        with open("teampulse_all_monthly.json", "w", encoding="utf-8") as f:
                            json.dump(all_monthly, f, indent=2, ensure_ascii=False, default=str)
                        print(f"   💾 Saved {len(all_monthly)} months to teampulse_all_monthly.json")

            except Exception as e:
                print(f"   ⚠️ Error: {e}")

    return True


def main():
    print("\n🚀 STARTING COMPREHENSIVE DATA EXTRACTION\n")

    extract_wall_of_tears()
    extract_teampulse()

    print("\n" + "=" * 80)
    print("✅ EXTRACTION COMPLETE")
    print("=" * 80)
    print("\nExtracted files:")
    print("  📁 Wall of Tears:")
    print("     - wall_stats.json")
    print("     - wall_months.json")
    print("     - wall_all_monthly.json")
    print("     - wall_feedbacks.json")
    print("  📁 TeamPulse:")
    print("     - teampulse_*.json")
    print()


if __name__ == "__main__":
    main()
