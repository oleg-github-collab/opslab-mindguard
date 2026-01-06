#!/usr/bin/env python3
"""
Скрипт для витягування даних зі Стіни плачу OpsLab
"""
import requests
import json
from datetime import datetime

BASE_URL = "https://opslab-feedback-production.up.railway.app"
LOGIN_EMAIL = "work.olegkaminskyi@gmail.com"
LOGIN_PASSWORD = "0000"

def fetch_wall_data():
    """Витягує всі дані зі Стіни плачу"""

    session = requests.Session()

    # Спроба авторізації
    print(f"🔐 Спроба входу на {BASE_URL}...")

    # Можливі endpoints для авторизації
    auth_endpoints = [
        "/api/auth/login",
        "/api/login",
        "/login",
        "/auth/login"
    ]

    login_data = {
        "email": LOGIN_EMAIL,
        "password": LOGIN_PASSWORD
    }

    auth_success = False
    for endpoint in auth_endpoints:
        try:
            url = f"{BASE_URL}{endpoint}"
            print(f"  Пробуємо: {url}")

            # POST запит для авторізації
            response = session.post(url, json=login_data, timeout=10)

            if response.status_code in [200, 201]:
                print(f"  ✅ Успішна авторізація через {endpoint}")
                auth_success = True

                # Зберігаємо токен якщо є
                try:
                    auth_data = response.json()
                    if "token" in auth_data:
                        session.headers.update({
                            "Authorization": f"Bearer {auth_data['token']}"
                        })
                    print(f"  Відповідь: {json.dumps(auth_data, indent=2, ensure_ascii=False)}")
                except:
                    pass
                break
        except Exception as e:
            print(f"  ❌ Помилка: {e}")

    if not auth_success:
        print("⚠️  Не вдалося авторизуватися через API, спробуємо публічні endpoints...")

    # Можливі endpoints для отримання даних
    data_endpoints = [
        "/api/feedback",
        "/api/posts",
        "/api/wall",
        "/api/complaints",
        "/feedback",
        "/posts"
    ]

    all_data = {
        "extraction_timestamp": datetime.now().isoformat(),
        "source": BASE_URL,
        "posts": [],
        "raw_responses": {}
    }

    # Намагаємось витягти дані
    for endpoint in data_endpoints:
        try:
            url = f"{BASE_URL}{endpoint}"
            print(f"\n📊 Намагаємось отримати дані: {url}")

            response = session.get(url, timeout=10)
            print(f"  Статус: {response.status_code}")

            if response.status_code == 200:
                try:
                    data = response.json()
                    all_data["raw_responses"][endpoint] = data
                    print(f"  ✅ Отримано дані: {len(json.dumps(data))} символів")

                    # Спроба парсингу структури
                    if isinstance(data, list):
                        all_data["posts"].extend(data)
                    elif isinstance(data, dict) and "posts" in data:
                        all_data["posts"].extend(data["posts"])
                    elif isinstance(data, dict) and "data" in data:
                        if isinstance(data["data"], list):
                            all_data["posts"].extend(data["data"])
                except Exception as e:
                    print(f"  ⚠️  Не JSON відповідь або помилка парсингу: {e}")
                    all_data["raw_responses"][endpoint] = response.text[:500]
        except Exception as e:
            print(f"  ❌ Помилка: {e}")

    # Зберігаємо результати
    output_file = "wall_data_extracted.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(all_data, f, ensure_ascii=False, indent=2)

    print(f"\n✅ Дані збережено у {output_file}")
    print(f"📈 Знайдено постів: {len(all_data['posts'])}")

    # Додатковий аналіз структури сайту
    print("\n🔍 Аналіз структури сайту...")
    try:
        main_page = session.get(BASE_URL, timeout=10)
        if main_page.status_code == 200:
            # Шукаємо API endpoints у HTML
            html_content = main_page.text

            # Пошук JavaScript fetch/axios викликів
            import re
            api_calls = re.findall(r'["\']/(api/[^"\']+)["\']', html_content)
            if api_calls:
                print("  Знайдені API endpoints у коді:")
                for call in set(api_calls):
                    print(f"    - {call}")
    except Exception as e:
        print(f"  ❌ Не вдалося проаналізувати головну сторінку: {e}")

    return all_data

if __name__ == "__main__":
    print("=" * 60)
    print("🔧 OpsLab Mindguard - Витягування даних зі Стіни плачу")
    print("=" * 60)

    try:
        data = fetch_wall_data()

        print("\n" + "=" * 60)
        print("✅ Витягування завершено!")
        print("=" * 60)

        if data["posts"]:
            print("\n📋 Приклад першого поста:")
            print(json.dumps(data["posts"][0], indent=2, ensure_ascii=False))
    except Exception as e:
        print(f"\n❌ Критична помилка: {e}")
        import traceback
        traceback.print_exc()
