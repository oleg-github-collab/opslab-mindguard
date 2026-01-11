use crate::crypto::Crypto;
use crate::domain::models::UserRole;
use anyhow::Result;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use sqlx::PgPool;
use uuid::Uuid;

struct SeedUser<'a> {
    code: &'a str,
    name: &'a str,
    email: &'a str,
    role: UserRole,
    note: &'a str,
}

pub async fn seed_all(pool: &PgPool, crypto: &Crypto) -> Result<()> {
    seed_questions(pool).await?;
    seed_users(pool, crypto).await?;
    Ok(())
}

async fn seed_users(pool: &PgPool, crypto: &Crypto) -> Result<()> {
    let users = vec![
        SeedUser {
            code: "0000",
            name: "Олег Камінський",
            email: "work.olegkaminskyi@gmail.com",
            role: UserRole::Admin,
            note: "Адмін",
        },
        SeedUser {
            code: "7139",
            name: "Jane Давидюк",
            email: "janedavydiuk@opslab.uk",
            role: UserRole::Founder,
            note: "Фаундерка (Participant + Viewer)",
        },
        SeedUser {
            code: "4582",
            name: "Вероніка Кухарчук",
            email: "veronika.kukharchuk@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "9267",
            name: "Михайло Іващук",
            email: "mykhailo.ivashchuk@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "3814",
            name: "Ірина Мячкова",
            email: "iryna.miachkova@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "8463",
            name: "Оксана Клінчаян",
            email: "oksana.klinchaian@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "6738",
            name: "Іванна Сакало",
            email: "ivanna.sakalo@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "1425",
            name: "Марія Василик",
            email: "mariya.vasylyk@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "1122",
            name: "Катерина Петухова",
            email: "kateryna.petukhova@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
    ];

    let argon = Argon2::default();
    for user in users {
        let salt = SaltString::generate(rand_core::OsRng);
        let hash = argon
            .hash_password(user.code.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();

        let enc_name = crypto.encrypt_str(user.name)?;
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, hash, enc_name, role, note)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (email) DO NOTHING
            "#,
            Uuid::new_v4(),
            user.email,
            hash,
            enc_name,
            user.role as UserRole,
            user.note
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_questions(pool: &PgPool) -> Result<()> {
    let existing: i64 = sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM questions")
        .fetch_one(pool)
        .await?;
    if existing > 0 {
        return Ok(());
    }

    let prompts = vec![
        "Який сьогодні рівень енергії?",
        "Чи відчували ви тривогу сьогодні?",
        "Як оцінюєте якість сну?",
        "Були проблеми з концентрацією?",
        "Наскільки ви відчували підтримку команди?",
        "Чи траплялися епізоди роздратування?",
        "Відчуття безнадійності сьогодні?",
        "Наскільки часто ви робили перерви?",
        "Чи виникали думки про уникнення завдань?",
        "Чи було бажання ізоляції від колег?",
        "Темп роботи: комфортний чи надмірний?",
        "Чи відчували ви сенс у роботі сьогодні?",
        "Рівень мотивації протягом дня?",
        "Чи було достатньо відпочинку?",
        "Чи були панічні епізоди?",
        "Чи відчували швидку втому?",
        "Чи були порушення апетиту?",
        "Наскільки ви задоволені прогресом?",
        "Чи отримали ви зворотній зв'язок від колег?",
        "Чи було важко розпочати робочий день?",
        "Чи відчували ви гордість за свою роботу сьогодні?",
    ];

    for (idx, prompt) in prompts.into_iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO questions (id, text, order_index)
            VALUES ($1, $2, $3)
            "#,
            (idx + 1) as i32,
            prompt,
            (idx + 1) as i32
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}
