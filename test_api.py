#!/usr/bin/env python3
import requests
import json

# Login
r = requests.post(
    "https://opslab-feedback-production.up.railway.app/api/auth/login",
    json={"email": "work.olegkaminskyi@gmail.com", "password": "QwertY24$"}
)

data = r.json()
token = data['access_token']
print(f"Token: {token[:50]}...")

headers = {"Authorization": f"Bearer {token}"}

# Try all possible endpoints
endpoints = [
    "/api/feedbacks",
    "/api/feedback",
    "/api/posts",
    "/api/wall",
    "/api/stats",
    "/api/stats/data",
    "/api/analytics",
    "/api/user/feedbacks",
]

for ep in endpoints:
    url = f"https://opslab-feedback-production.up.railway.app{ep}"
    resp = requests.get(url, headers=headers)

    print(f"\n{ep}")
    print(f"  Status: {resp.status_code}")
    print(f"  Length: {len(resp.text)}")

    # Check if it's JSON
    if resp.status_code == 200:
        try:
            data = resp.json()
            print(f"  ✅ JSON! Type: {type(data)}")

            if isinstance(data, list):
                print(f"  Items: {len(data)}")
                if data:
                    print(f"  First item keys: {list(data[0].keys())}")
            elif isinstance(data, dict):
                print(f"  Keys: {list(data.keys())}")

            # Save it
            filename = f"api_{ep.replace('/', '_').strip('_')}.json"
            with open(filename, 'w', encoding='utf-8') as f:
                json.dump(data, f, indent=2, ensure_ascii=False, default=str)
            print(f"  💾 Saved to {filename}")

        except:
            print(f"  ❌ Not JSON (HTML SPA)")
