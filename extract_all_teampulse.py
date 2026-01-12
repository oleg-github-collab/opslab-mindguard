#!/usr/bin/env python3
"""
Extract ALL DATA from teampulse - all months, all users, all metrics
"""

import requests
import json

BASE = "https://teampulse-mindguard-production.up.railway.app"

def extract_all_data():
    """Extract everything available from teampulse"""

    print("=" * 80)
    print("EXTRACTING ALL TEAMPULSE DATA")
    print("=" * 80)

    all_data = {
        "metrics": {},
        "monthly": {},
        "users": [],
        "raw_responses": {}
    }

    # 1. Get current metrics (already worked!)
    print("\n📊 Fetching current metrics...")
    resp = requests.get(f"{BASE}/api/metrics")
    if resp.status_code == 200:
        all_data["metrics"]["current"] = resp.json()
        print(f"   ✅ Got current metrics")
        print(f"      Well-being: {all_data['metrics']['current']['wellbeingIndex']}")
        print(f"      Burnout: {all_data['metrics']['current']['burnoutIndex']}%")
        print(f"      At Risk: {all_data['metrics']['current']['atRiskCount']} users")
        print(f"      Critical: {all_data['metrics']['current']['criticalCount']} users")
    else:
        print(f"   ❌ Failed: {resp.status_code}")

    # 2. Try to get monthly data for each month (8-12 as seen in dashboard)
    months = [8, 9, 10, 11, 12]
    month_names = {8: "Серпень", 9: "Вересень", 10: "Жовтень", 11: "Листопад", 12: "Грудень"}

    for month in months:
        print(f"\n📅 Trying to fetch {month_names[month]} ({month}/2025)...")

        # Try different endpoint patterns
        endpoints = [
            f"/api/metrics?month={month}",
            f"/api/metrics/{month}",
            f"/api/data/month/{month}",
            f"/api/monthly/{month}",
            f"/api/stats/month/{month}",
        ]

        for ep in endpoints:
            resp = requests.get(f"{BASE}{ep}")
            if resp.status_code == 200:
                try:
                    data = resp.json()
                    all_data["monthly"][month] = data
                    print(f"   ✅ {ep} -> Success!")
                    break
                except:
                    pass

    # 3. Try to get individual user data
    print("\n👥 Trying to fetch users data...")

    user_endpoints = [
        "/api/users",
        "/api/users/list",
        "/api/team",
        "/api/team/members",
        "/api/dashboard/users",
    ]

    for ep in user_endpoints:
        resp = requests.get(f"{BASE}{ep}")
        if resp.status_code == 200:
            try:
                data = resp.json()
                all_data["users"] = data
                print(f"   ✅ {ep} -> Got {len(data) if isinstance(data, list) else '?'} users")
                break
            except:
                pass

    # 4. Try all other possible endpoints
    print("\n🔍 Scanning for other data endpoints...")

    other_endpoints = [
        "/api/dashboard",
        "/api/dashboard/data",
        "/api/analytics",
        "/api/reports",
        "/api/checkins",
        "/api/history",
        "/api/trends",
        "/api/alerts",
        "/api/risks",
        "/api/data",
        "/api/all",
        "/api/export",
    ]

    for ep in other_endpoints:
        resp = requests.get(f"{BASE}{ep}")
        if resp.status_code == 200:
            try:
                data = resp.json()
                all_data["raw_responses"][ep] = data
                print(f"   ✅ {ep} -> Success!")
            except:
                pass

    # 5. Save everything
    print("\n💾 Saving all extracted data...")

    with open("TEAMPULSE_ALL_DATA.json", "w", encoding="utf-8") as f:
        json.dump(all_data, f, indent=2, ensure_ascii=False, default=str)

    print("   ✅ Saved to TEAMPULSE_ALL_DATA.json")

    # 6. Print summary
    print("\n" + "=" * 80)
    print("EXTRACTION SUMMARY")
    print("=" * 80)
    print(f"Current Metrics: {'✅' if all_data['metrics'].get('current') else '❌'}")
    print(f"Monthly Data: {len(all_data['monthly'])} months")
    print(f"Users: {len(all_data['users']) if isinstance(all_data['users'], list) else '❌'}")
    print(f"Other Endpoints: {len(all_data['raw_responses'])}")

    if all_data['metrics'].get('current'):
        print("\n📊 Current Team Health:")
        m = all_data['metrics']['current']
        print(f"   Well-being Index: {m.get('wellbeingIndex', '?')}")
        print(f"   Depression Level: {m.get('depressionLevel', '?')}")
        print(f"   Anxiety Level: {m.get('anxietyLevel', '?')}")
        print(f"   Burnout Index: {m.get('burnoutIndex', '?')}%")
        print(f"   Sleep Duration: {m.get('sleepDuration', '?')} hours")
        print(f"   Sleep Quality: {m.get('sleepQuality', '?')}/10")
        print(f"   Work-Life Balance: {m.get('workLifeBalance', '?')}/10")
        print(f"   Stress Level: {m.get('stressLevel', '?')}")
        print(f"   At Risk: {m.get('atRiskCount', '?')} users")
        print(f"   Critical: {m.get('criticalCount', '?')} users")

    print("\n✅ COMPLETE!")
    return all_data

if __name__ == "__main__":
    extract_all_data()
