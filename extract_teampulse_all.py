#!/usr/bin/env python3
"""
COMPREHENSIVE extraction from teampulse-mindguard-production.up.railway.app
Try ALL possible authentication and API patterns
"""

import requests
import json

EMAIL = "work.olegkaminskyi@gmail.com"
PASSWORD = "QwertY24$"
BASE = "https://teampulse-mindguard-production.up.railway.app"

def try_all_login_methods():
    """Try every possible login method"""

    login_attempts = [
        # Different endpoints
        {"method": "POST", "url": f"{BASE}/api/auth/login", "data": {"email": EMAIL, "password": PASSWORD}},
        {"method": "POST", "url": f"{BASE}/api/auth/login", "data": {"email": EMAIL, "code": PASSWORD}},
        {"method": "POST", "url": f"{BASE}/auth/login", "data": {"email": EMAIL, "password": PASSWORD}},
        {"method": "POST", "url": f"{BASE}/auth/login", "data": {"email": EMAIL, "code": PASSWORD}},
        {"method": "POST", "url": f"{BASE}/login", "data": {"email": EMAIL, "password": PASSWORD}},
        {"method": "POST", "url": f"{BASE}/login", "data": {"email": EMAIL, "code": PASSWORD}},

        # Try with 4-digit code
        {"method": "POST", "url": f"{BASE}/api/auth/login", "data": {"email": EMAIL, "code": "0000"}},
        {"method": "POST", "url": f"{BASE}/auth/login", "data": {"email": EMAIL, "code": "0000"}},

        # Try session-based login
        {"method": "POST", "url": f"{BASE}/api/session/login", "data": {"email": EMAIL, "password": PASSWORD}},
        {"method": "POST", "url": f"{BASE}/session/login", "data": {"email": EMAIL, "code": "0000"}},
    ]

    print("=" * 80)
    print("TRYING ALL LOGIN METHODS")
    print("=" * 80)

    for attempt in login_attempts:
        print(f"\n{attempt['method']} {attempt['url']}")
        print(f"  Payload: {attempt['data']}")

        try:
            resp = requests.post(attempt['url'], json=attempt['data'], headers={"Content-Type": "application/json"})
            print(f"  Status: {resp.status_code}")

            if resp.status_code == 200:
                print(f"  ✅ SUCCESS! Response: {resp.text[:200]}")

                # Try to parse JSON
                try:
                    data = resp.json()
                    print(f"  Keys: {list(data.keys())}")

                    # Save successful login
                    with open("teampulse_login_success.json", "w") as f:
                        json.dump({
                            "endpoint": attempt['url'],
                            "payload": attempt['data'],
                            "response": data
                        }, f, indent=2)

                    return resp, data
                except:
                    print(f"  Response is not JSON")
            else:
                print(f"  ❌ {resp.status_code}: {resp.text[:100]}")

        except Exception as e:
            print(f"  💥 Error: {e}")

    return None, None

def try_direct_access():
    """Try accessing data endpoints without login (might be open)"""

    print("\n" + "=" * 80)
    print("TRYING DIRECT ACCESS (NO AUTH)")
    print("=" * 80)

    endpoints = [
        "/api/stats",
        "/api/data",
        "/api/metrics",
        "/api/users",
        "/api/health",
        "/api/feedback",
        "/stats",
        "/data",
        "/health",
    ]

    for ep in endpoints:
        url = f"{BASE}{ep}"
        print(f"\nGET {url}")

        try:
            resp = requests.get(url)
            print(f"  Status: {resp.status_code}")

            if resp.status_code == 200:
                try:
                    data = resp.json()
                    print(f"  ✅ JSON! Type: {type(data).__name__}")

                    filename = f"teampulse_direct_{ep.replace('/', '_')}.json"
                    with open(filename, "w", encoding="utf-8") as f:
                        json.dump(data, f, indent=2, ensure_ascii=False, default=str)
                    print(f"  💾 Saved to {filename}")
                except:
                    print(f"  HTML: {resp.text[:100]}")
        except Exception as e:
            print(f"  💥 Error: {e}")

def check_if_same_as_new_platform():
    """Check if teampulse IS the new platform"""

    print("\n" + "=" * 80)
    print("CHECKING IF TEAMPULSE IS THE NEW PLATFORM")
    print("=" * 80)

    # Try logging in with session cookie approach (like our current backend)
    session = requests.Session()

    # Try the same login as our new backend
    print("\nTrying session-based login (like new backend)...")
    resp = session.post(
        f"{BASE}/auth/login",
        json={"email": EMAIL, "code": "0000"},
        headers={"Content-Type": "application/json"}
    )

    print(f"Status: {resp.status_code}")
    print(f"Cookies: {dict(session.cookies)}")
    print(f"Response: {resp.text[:200]}")

    if resp.status_code == 200:
        # Try accessing dashboard
        print("\n✅ Login successful! Trying to access dashboard...")

        dash_resp = session.get(f"{BASE}/dashboard/user")
        print(f"Dashboard status: {dash_resp.status_code}")

        if dash_resp.status_code == 200:
            try:
                data = dash_resp.json()
                print(f"Dashboard data: {list(data.keys())}")

                with open("teampulse_dashboard.json", "w") as f:
                    json.dump(data, f, indent=2, ensure_ascii=False, default=str)
            except:
                print(f"Dashboard HTML: {dash_resp.text[:200]}")

def main():
    print("\n🚀 COMPREHENSIVE TEAMPULSE DATA EXTRACTION\n")

    # Step 1: Try all login methods
    session, login_data = try_all_login_methods()

    # Step 2: Try direct access
    try_direct_access()

    # Step 3: Check if it's the new platform
    check_if_same_as_new_platform()

    # Step 4: If login succeeded, extract everything
    if session and login_data:
        print("\n" + "=" * 80)
        print("EXTRACTING ALL DATA (AUTHENTICATED)")
        print("=" * 80)

        # Get token if JWT-based
        token = login_data.get('access_token')
        headers = {}
        if token:
            headers['Authorization'] = f"Bearer {token}"

        # Try all possible data endpoints
        endpoints = [
            "/api/stats",
            "/api/stats/available-months",
            "/api/feedback",
            "/api/users",
            "/api/dashboard",
            "/api/metrics",
            "/api/monthly",
            "/dashboard/user",
            "/admin/heatmap",
        ]

        for ep in endpoints:
            print(f"\n📊 {ep}")
            resp = requests.get(f"{BASE}{ep}", headers=headers)
            print(f"   Status: {resp.status_code}")

            if resp.status_code == 200:
                try:
                    data = resp.json()
                    filename = f"teampulse_{ep.replace('/', '_').strip('_')}.json"
                    with open(filename, "w", encoding="utf-8") as f:
                        json.dump(data, f, indent=2, ensure_ascii=False, default=str)
                    print(f"   ✅ Saved to {filename}")
                except:
                    pass

    print("\n" + "=" * 80)
    print("✅ EXTRACTION COMPLETE")
    print("=" * 80)

if __name__ == "__main__":
    main()
