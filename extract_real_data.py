#!/usr/bin/env python3
"""
Extract REAL data from old OpsLab systems using discovered API endpoints.
"""

import requests
import json
from datetime import datetime

EMAIL = "work.olegkaminskyi@gmail.com"
PASSWORD = "QwertY24$"

def extract_wall_of_tears():
    """Extract wall of tears data"""
    print("=" * 70)
    print("EXTRACTING WALL OF TEARS DATA")
    print("=" * 70)

    base = "https://opslab-feedback-production.up.railway.app"
    s = requests.Session()

    # Login
    print(f"\n📝 Logging in as {EMAIL}...")
    login_resp = s.post(
        f"{base}/auth/login",
        json={"email": EMAIL, "password": PASSWORD},
        headers={"Content-Type": "application/json"}
    )

    print(f"   Status: {login_resp.status_code}")

    if login_resp.status_code != 200:
        print(f"   ❌ Login failed: {login_resp.text[:300]}")

        # Try with 'code' instead of 'password'
        print("\n   Trying with 'code' field...")
        login_resp = s.post(
            f"{base}/auth/login",
            json={"email": EMAIL, "code": PASSWORD},
            headers={"Content-Type": "application/json"}
        )
        print(f"   Status: {login_resp.status_code}")

        if login_resp.status_code != 200:
            print(f"   ❌ Still failed")
            return None

    print(f"   ✅ Login successful!")
    print(f"   Cookies: {list(s.cookies.keys())}")

    # Get stats
    print("\n📊 Fetching /stats...")
    stats_resp = s.get(f"{base}/stats")
    print(f"   Status: {stats_resp.status_code}")

    if stats_resp.status_code == 200:
        try:
            stats = stats_resp.json()
            print(f"   ✅ Got stats: {list(stats.keys())}")

            with open('wall_stats.json', 'w', encoding='utf-8') as f:
                json.dump(stats, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_stats.json")
        except:
            print(f"   ⚠️  Not JSON: {stats_resp.text[:200]}")

    # Get available months
    print("\n📅 Fetching /stats/available-months...")
    months_resp = s.get(f"{base}/stats/available-months")
    print(f"   Status: {months_resp.status_code}")

    if months_resp.status_code == 200:
        try:
            months = months_resp.json()
            print(f"   ✅ Got {len(months)} months")

            with open('wall_months.json', 'w', encoding='utf-8') as f:
                json.dump(months, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_months.json")

            # Get data for each month
            all_monthly = []
            for month in months:
                print(f"\n   📦 Fetching data for {month}...")
                month_resp = s.get(f"{base}/stats?month={month}")

                if month_resp.status_code == 200:
                    try:
                        month_data = month_resp.json()
                        all_monthly.append({
                            'month': month,
                            'data': month_data
                        })
                        print(f"      ✅ Got data")
                    except:
                        print(f"      ❌ Not JSON")
                else:
                    print(f"      ❌ Failed: {month_resp.status_code}")

            if all_monthly:
                with open('wall_all_months.json', 'w', encoding='utf-8') as f:
                    json.dump(all_monthly, f, indent=2, ensure_ascii=False, default=str)
                print(f"\n   💾 Saved all monthly data to wall_all_months.json")

        except:
            print(f"   ⚠️  Not JSON: {months_resp.text[:200]}")

    # Get feedbacks
    print("\n💬 Fetching /feedbacks...")
    feedbacks_resp = s.get(f"{base}/feedbacks")
    print(f"   Status: {feedbacks_resp.status_code}")

    if feedbacks_resp.status_code == 200:
        try:
            feedbacks = feedbacks_resp.json()
            print(f"   ✅ Got {len(feedbacks) if isinstance(feedbacks, list) else '?'} feedbacks")

            with open('wall_feedbacks.json', 'w', encoding='utf-8') as f:
                json.dump(feedbacks, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_feedbacks.json")
        except:
            print(f"   ⚠️  Not JSON: {feedbacks_resp.text[:200]}")

    # Get feedback (singular)
    print("\n💬 Fetching /feedback...")
    feedback_resp = s.get(f"{base}/feedback")
    print(f"   Status: {feedback_resp.status_code}")

    if feedback_resp.status_code == 200:
        try:
            feedback = feedback_resp.json()
            print(f"   ✅ Got feedback data")

            with open('wall_feedback.json', 'w', encoding='utf-8') as f:
                json.dump(feedback, f, indent=2, ensure_ascii=False, default=str)
            print(f"   💾 Saved to wall_feedback.json")
        except:
            print(f"   ⚠️  Not JSON: {feedback_resp.text[:200]}")

    return True

def extract_teampulse():
    """Extract monthly teampulse data"""
    print("\n" + "=" * 70)
    print("EXTRACTING TEAMPULSE DATA")
    print("=" * 70)

    base = "https://teampulse-mindguard-production.up.railway.app"
    s = requests.Session()

    # Try different login endpoints
    login_endpoints = [
        "/auth/login",
        "/api/auth/login",
        "/login"
    ]

    logged_in = False

    for endpoint in login_endpoints:
        print(f"\n📝 Trying login at {endpoint}...")

        for payload in [
            {"email": EMAIL, "password": PASSWORD},
            {"email": EMAIL, "code": PASSWORD}
        ]:
            resp = s.post(
                f"{base}{endpoint}",
                json=payload,
                headers={"Content-Type": "application/json"}
            )

            print(f"   {list(payload.keys())} -> Status: {resp.status_code}")

            if resp.status_code == 200:
                try:
                    data = resp.json()
                    print(f"   ✅ Login successful! {list(data.keys())}")
                    logged_in = True
                    break
                except:
                    print(f"   Response: {resp.text[:200]}")

        if logged_in:
            break

    if not logged_in:
        print("\n   ❌ Could not login to teampulse")
        return None

    print(f"   Cookies: {list(s.cookies.keys())}")

    # Try to find data endpoints
    endpoints = [
        "/stats",
        "/api/stats",
        "/stats/available-months",
        "/api/stats/available-months",
        "/data",
        "/api/data",
        "/dashboard",
        "/api/dashboard"
    ]

    for ep in endpoints:
        print(f"\n📊 Trying {ep}...")
        resp = s.get(f"{base}{ep}")
        print(f"   Status: {resp.status_code}")

        if resp.status_code == 200:
            try:
                data = resp.json()
                filename = f"teampulse_{ep.replace('/', '_').strip('_')}.json"
                with open(filename, 'w', encoding='utf-8') as f:
                    json.dump(data, f, indent=2, ensure_ascii=False, default=str)
                print(f"   ✅ Saved to {filename}")
            except:
                print(f"   ⚠️  Not JSON")

    return True

def main():
    print("\n🚀 STARTING DATA EXTRACTION\n")

    extract_wall_of_tears()
    extract_teampulse()

    print("\n" + "=" * 70)
    print("✅ EXTRACTION COMPLETE")
    print("=" * 70)
    print("\nCheck the following files:")
    print("  - wall_stats.json")
    print("  - wall_months.json")
    print("  - wall_all_months.json")
    print("  - wall_feedbacks.json")
    print("  - wall_feedback.json")
    print("  - teampulse_*.json")
    print()

if __name__ == "__main__":
    main()
