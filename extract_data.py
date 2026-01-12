#!/usr/bin/env python3
"""
Extract data from old OpsLab systems:
1. Wall of tears from https://opslab-feedback-production.up.railway.app
2. Monthly data from https://teampulse-mindguard-production.up.railway.app
"""

import requests
import json
from datetime import datetime

# Credentials
EMAIL = "work.olegkaminskyi@gmail.com"
PASSWORD = "QwertY24$"

def extract_wall_data():
    """Extract wall of tears data from old feedback system"""
    print("=" * 60)
    print("EXTRACTING WALL OF TEARS DATA")
    print("=" * 60)

    base_url = "https://opslab-feedback-production.up.railway.app"
    session = requests.Session()

    # Step 1: Login
    print(f"\n1. Logging in as {EMAIL}...")
    login_response = session.post(
        f"{base_url}/auth/login",
        json={"email": EMAIL, "code": PASSWORD},
        headers={"Content-Type": "application/json"}
    )

    print(f"   Status: {login_response.status_code}")
    print(f"   Response: {login_response.text[:200]}")
    print(f"   Cookies: {dict(session.cookies)}")

    if login_response.status_code != 200:
        print("   ❌ Login failed!")
        return None

    print("   ✅ Login successful!")

    # Step 2: Get wall data
    print("\n2. Fetching wall posts...")
    wall_response = session.get(f"{base_url}/feedback/wall")

    print(f"   Status: {wall_response.status_code}")
    print(f"   Response length: {len(wall_response.text)}")
    print(f"   Content-Type: {wall_response.headers.get('content-type')}")

    if wall_response.status_code != 200:
        print(f"   ❌ Failed to get wall data: {wall_response.text[:500]}")
        return None

    # Check if response is JSON
    if 'application/json' in wall_response.headers.get('content-type', ''):
        wall_data = wall_response.json()
        print(f"   ✅ Got {len(wall_data.get('posts', []))} posts")
    else:
        print(f"   ⚠️  Response is HTML, not JSON")
        print(f"   First 500 chars:\n{wall_response.text[:500]}")

        # Try API endpoints
        print("\n   Trying /api/feedback/wall...")
        api_response = session.get(f"{base_url}/api/feedback/wall")
        print(f"   Status: {api_response.status_code}")

        if api_response.status_code == 200 and 'application/json' in api_response.headers.get('content-type', ''):
            wall_data = api_response.json()
            print(f"   ✅ Got {len(wall_data.get('posts', []))} posts from API")
        else:
            print(f"   ❌ No JSON API found")
            wall_data = {'posts': [], 'html': wall_response.text}
            print(f"   Saving HTML response for manual inspection")

    # Save to file
    with open('wall_data.json', 'w', encoding='utf-8') as f:
        json.dump(wall_data, f, indent=2, ensure_ascii=False, default=str)

    print(f"\n   💾 Saved to wall_data.json")

    return wall_data

def extract_teampulse_data():
    """Extract monthly data from old teampulse system"""
    print("\n" + "=" * 60)
    print("EXTRACTING TEAMPULSE MONTHLY DATA")
    print("=" * 60)

    base_url = "https://teampulse-mindguard-production.up.railway.app"
    session = requests.Session()

    # Step 1: Login
    print(f"\n1. Logging in as {EMAIL}...")
    login_response = session.post(
        f"{base_url}/auth/login",
        json={"email": EMAIL, "code": PASSWORD},
        headers={"Content-Type": "application/json"}
    )

    print(f"   Status: {login_response.status_code}")
    print(f"   Response: {login_response.text[:200]}")
    print(f"   Cookies: {dict(session.cookies)}")

    if login_response.status_code != 200:
        print("   ❌ Login failed!")
        return None

    login_data = login_response.json()
    print("   ✅ Login successful!")
    print(f"   User ID: {login_data.get('user_id')}")
    print(f"   Role: {login_data.get('role')}")

    user_id = login_data.get('user_id')

    # Step 2: Get user history
    print("\n2. Fetching user history...")
    history_response = session.get(f"{base_url}/dashboard/user/{user_id}/history")

    print(f"   Status: {history_response.status_code}")
    print(f"   Response length: {len(history_response.text)}")

    if history_response.status_code != 200:
        print(f"   ❌ Failed to get history: {history_response.text[:500]}")
        return None

    history_data = history_response.json()
    print(f"   ✅ Got {len(history_data.get('months', []))} months of data")

    # Step 3: Get admin heatmap (all users data)
    print("\n3. Fetching admin heatmap (all users)...")
    heatmap_response = session.get(f"{base_url}/admin/heatmap")

    print(f"   Status: {heatmap_response.status_code}")

    if heatmap_response.status_code == 200:
        heatmap_data = heatmap_response.json()
        print(f"   ✅ Got data for {len(heatmap_data.get('users', []))} users")

        # Save heatmap
        with open('heatmap_data.json', 'w', encoding='utf-8') as f:
            json.dump(heatmap_data, f, indent=2, ensure_ascii=False, default=str)
        print(f"   💾 Saved to heatmap_data.json")
    else:
        print(f"   ⚠️  No admin access: {heatmap_response.text[:200]}")
        heatmap_data = None

    # Save history
    with open('history_data.json', 'w', encoding='utf-8') as f:
        json.dump(history_data, f, indent=2, ensure_ascii=False, default=str)

    print(f"   💾 Saved to history_data.json")

    # Step 4: Try to get all users history if admin
    all_history = {"users": []}

    if login_data.get('role') in ['Admin', 'Founder']:
        print("\n4. Fetching all users history (admin access)...")

        if heatmap_data:
            for user in heatmap_data.get('users', []):
                uid = user['user_id']
                print(f"   Fetching history for {user['name']} ({uid})...")

                user_hist_response = session.get(f"{base_url}/dashboard/user/{uid}/history")
                if user_hist_response.status_code == 200:
                    user_hist = user_hist_response.json()
                    all_history['users'].append({
                        'user_id': uid,
                        'name': user['name'],
                        'email': user.get('email', 'unknown'),
                        'history': user_hist
                    })
                    print(f"      ✅ {len(user_hist.get('months', []))} months")
                else:
                    print(f"      ❌ Failed")

        # Save all history
        with open('all_users_history.json', 'w', encoding='utf-8') as f:
            json.dump(all_history, f, indent=2, ensure_ascii=False, default=str)

        print(f"\n   💾 Saved all users history to all_users_history.json")

    return {
        'history': history_data,
        'heatmap': heatmap_data,
        'all_history': all_history
    }

def main():
    print("🚀 Starting data extraction...\n")

    # Extract wall data
    wall_data = extract_wall_data()

    # Extract teampulse data
    teampulse_data = extract_teampulse_data()

    print("\n" + "=" * 60)
    print("EXTRACTION COMPLETE")
    print("=" * 60)
    print("\nFiles created:")
    print("  - wall_data.json (wall of tears posts)")
    print("  - history_data.json (your monthly data)")
    print("  - heatmap_data.json (all users current state)")
    print("  - all_users_history.json (all users monthly data)")
    print("\n✅ Done!")

if __name__ == "__main__":
    main()
