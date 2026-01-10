///! Weekly Summary System (#6 + #10)
///! Відправляє щоп'ятниці о 17:00 детальний summary з метриками та team benchmark

use crate::bot::daily_checkin::Metrics;
use crate::db::{self, TeamAverage};
use crate::state::SharedState;
use anyhow::Result;
use chrono::{Duration, Utc};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use uuid::Uuid;

pub struct WeeklySummary {
    pub user_id: Uuid,
    pub week_start: chrono::DateTime<Utc>,
    pub week_end: chrono::DateTime<Utc>,
    pub current_metrics: Metrics,
    pub previous_metrics: Option<Metrics>,
    pub checkin_count: i32,
    pub streak: i32,
    pub team_average: TeamAverage,
    pub kudos_count: i64,
}

impl WeeklySummary {
    pub async fn generate(pool: &sqlx::PgPool, user_id: Uuid) -> Result<Self> {
        let now = Utc::now();
        let week_start = now - Duration::days(7);

        // Поточні метрики (цей тиждень)
        let current_metrics = match db::calculate_user_metrics(pool, user_id).await? {
            Some(m) => m,
            None => {
                // Якщо немає даних, повернути дефолтні значення
                Metrics {
                    who5_score: 0.0,
                    phq9_score: 0.0,
                    gad7_score: 0.0,
                    mbi_score: 0.0,
                    sleep_duration: 0.0,
                    work_life_balance: 0.0,
                    stress_level: 0.0,
                }
            }
        };

        // Попередній тиждень для порівняння
        let previous_metrics = db::calculate_user_metrics_for_period(
            pool,
            user_id,
            week_start - Duration::days(7),
            week_start,
        )
        .await
        .ok()
        .flatten();

        // Кількість check-ins
        let checkin_count = db::get_checkin_count_for_week(pool, user_id).await?;

        // Streak
        let streak = db::get_user_current_streak(pool, user_id).await?;

        // Team average (анонімно) - #10 Feature
        let team_average = db::get_team_average_metrics(pool).await?;

        // Kudos count (#17)
        let kudos_count = db::get_kudos_count_for_week(pool, user_id).await?;

        Ok(Self {
            user_id,
            week_start,
            week_end: now,
            current_metrics,
            previous_metrics,
            checkin_count,
            streak,
            team_average,
            kudos_count,
        })
    }

    pub async fn format_telegram_message(&self, pool: &sqlx::PgPool, crypto: &crate::crypto::Crypto) -> Result<String> {
        let mut msg = String::from("📊 *ТВІЙ ТИЖНЕВИЙ SUMMARY*\n\n");

        // Check-ins & Streak
        msg.push_str(&format!("✅ Чекінів: *{}/7*\n", self.checkin_count));
        msg.push_str(&format!("🔥 Streak: *{} днів*\n", self.streak));

        if self.kudos_count > 0 {
            msg.push_str(&format!("🎉 Kudos отримано: *{}*\n", self.kudos_count));
        }

        msg.push_str("\n");

        // Mental Health Metrics
        msg.push_str("🧠 *Ментальне здоров'я:*\n\n");

        // WHO-5 Well-being
        msg.push_str(&format!(
            "💚 WHO-5 Well-being: *{:.1}/100* {}\n",
            self.current_metrics.who5_score,
            self.get_trend_emoji("who5")
        ));
        msg.push_str(&self.get_who5_interpretation());
        msg.push_str("\n");

        // PHQ-9 Depression
        msg.push_str(&format!(
            "🧠 PHQ-9 Depression: *{:.1}/27* {}\n",
            self.current_metrics.phq9_score,
            self.get_trend_emoji("phq9")
        ));
        msg.push_str(&self.get_phq9_interpretation());
        msg.push_str("\n");

        // GAD-7 Anxiety
        msg.push_str(&format!(
            "😰 GAD-7 Anxiety: *{:.1}/21* {}\n",
            self.current_metrics.gad7_score,
            self.get_trend_emoji("gad7")
        ));
        msg.push_str(&self.get_gad7_interpretation());
        msg.push_str("\n");

        // Burnout
        msg.push_str(&format!(
            "🔥 Burnout Risk: *{:.0}%* {}\n",
            self.current_metrics.burnout_percentage(),
            self.get_trend_emoji("burnout")
        ));
        msg.push_str(&self.get_burnout_interpretation());
        msg.push_str("\n\n");

        // #10 Team Benchmark (Anonymous)
        msg.push_str("📈 *Порівняння з командою (анонімно):*\n");
        msg.push_str(&self.format_team_comparison());
        msg.push_str("\n\n");

        // Insights
        msg.push_str("💡 *Інсайти тижня:*\n");
        msg.push_str(&self.generate_insights());
        msg.push_str("\n");

        // Kudos section if received any
        if self.kudos_count > 0 {
            msg.push_str("\n🎉 *Kudos від колег:*\n");
            let kudos_list = db::get_recent_kudos(pool, self.user_id, 3).await?;
            for kudos in kudos_list {
                let from_name = crypto
                    .decrypt_str(&kudos.from_user_enc_name)
                    .unwrap_or_else(|_| "Colleague".to_string());
                msg.push_str(&format!("• \"{}\" - _{}_\n", kudos.message, from_name));
            }
        }

        msg.push_str("\n_Продовжуй в тому ж дусі! 💪_\n");
        msg.push_str("_Наступний summary - в п'ятницю!_");

        Ok(msg)
    }

    fn get_trend_emoji(&self, metric: &str) -> &'static str {
        if let Some(prev) = &self.previous_metrics {
            let (current, previous) = match metric {
                "who5" => (self.current_metrics.who5_score, prev.who5_score),
                "phq9" => (self.current_metrics.phq9_score, prev.phq9_score),
                "gad7" => (self.current_metrics.gad7_score, prev.gad7_score),
                "burnout" => (
                    self.current_metrics.burnout_percentage(),
                    prev.burnout_percentage(),
                ),
                _ => return "→",
            };

            let diff = current - previous;

            // WHO-5: вище = краще
            if metric == "who5" {
                if diff > 5.0 {
                    "📈"
                } else if diff < -5.0 {
                    "📉"
                } else {
                    "→"
                }
            } else {
                // PHQ-9, GAD-7, burnout: нижче = краще
                if diff < -2.0 {
                    "📈"
                } else if diff > 2.0 {
                    "📉"
                } else {
                    "→"
                }
            }
        } else {
            "→"
        }
    }

    fn get_who5_interpretation(&self) -> &'static str {
        let score = self.current_metrics.who5_score;
        if score >= 75.0 {
            "  ✨ Відмінний рівень!\n"
        } else if score >= 50.0 {
            "  ✅ Нормальний рівень\n"
        } else if score >= 35.0 {
            "  ⚠️ Знижений - потрібна увага\n"
        } else {
            "  🚨 Критично низький - потрібна допомога!\n"
        }
    }

    fn get_phq9_interpretation(&self) -> &'static str {
        let score = self.current_metrics.phq9_score;
        if score < 5.0 {
            "  ✅ Мінімальні симптоми\n"
        } else if score < 10.0 {
            "  ⚠️ Легкі симптоми\n"
        } else if score < 15.0 {
            "  ⚠️ Помірні симптоми - поговори з кимось\n"
        } else if score < 20.0 {
            "  🚨 Значні симптоми - потрібна допомога\n"
        } else {
            "  🚨 Важкі симптоми - негайно зверніться до фахівця!\n"
        }
    }

    fn get_gad7_interpretation(&self) -> &'static str {
        let score = self.current_metrics.gad7_score;
        if score < 5.0 {
            "  ✅ Мінімальна тривога\n"
        } else if score < 10.0 {
            "  ⚠️ Легка тривога\n"
        } else if score < 15.0 {
            "  ⚠️ Помірна тривога - meditation допоможе\n"
        } else {
            "  🚨 Важка тривога - потрібна підтримка!\n"
        }
    }

    fn get_burnout_interpretation(&self) -> &'static str {
        let score = self.current_metrics.burnout_percentage();
        if score < 30.0 {
            "  ✅ Низький ризик\n"
        } else if score < 50.0 {
            "  ⚠️ Помірний ризик\n"
        } else if score < 70.0 {
            "  🚨 Високий ризик - візьми break!\n"
        } else {
            "  🚨 Критичний ризик - потрібна відпустка!\n"
        }
    }

    fn format_team_comparison(&self) -> String {
        let mut comp = String::new();

        let who5_diff = self.current_metrics.who5_score - self.team_average.who5;
        let phq9_diff = self.current_metrics.phq9_score - self.team_average.phq9;
        let gad7_diff = self.current_metrics.gad7_score - self.team_average.gad7;

        comp.push_str(&format!(
            "• WHO-5: {} ({:+.1})\n",
            if who5_diff > 0.0 {
                "вище середнього ✨"
            } else {
                "нижче середнього"
            },
            who5_diff
        ));

        comp.push_str(&format!(
            "• PHQ-9: {} ({:+.1})\n",
            if phq9_diff < 0.0 {
                "краще команди ✨"
            } else {
                "гірше команди"
            },
            phq9_diff
        ));

        comp.push_str(&format!(
            "• GAD-7: {} ({:+.1})",
            if gad7_diff < 0.0 {
                "менше тривоги ✨"
            } else {
                "більше тривоги"
            },
            gad7_diff
        ));

        comp
    }

    fn generate_insights(&self) -> String {
        let mut insights = Vec::new();

        if self.current_metrics.who5_score >= 75.0 {
            insights.push("• Твій well-being на високому рівні! 🎉");
        } else if self.current_metrics.who5_score < 50.0 {
            insights.push("• Well-being низький. Поговори з Jane або керівником 💙");
        }

        if self.streak >= 7 {
            insights.push(&format!("• {} днів streak! Ти супер! 🔥", self.streak));
        } else if self.checkin_count < 5 {
            insights.push("• Спробуй проходити чекіни частіше для точнішої картини");
        }

        if self.current_metrics.phq9_score < 5.0 {
            insights.push("• Депресивні симптоми мінімальні ✨");
        } else if self.current_metrics.phq9_score >= 15.0 {
            insights.push("• ⚠️ Високий рівень депресивних симптомів - не ігноруй це!");
        }

        if self.current_metrics.burnout_percentage() < 30.0 {
            insights.push("• Ризик burnout низький 💚");
        } else if self.current_metrics.burnout_percentage() > 70.0 {
            insights.push("• ⚠️ Високий ризик burnout! Потрібна перерва негайно");
        }

        if self.current_metrics.stress_level > 30.0 {
            insights.push("• Високий stress - спробуй meditation 4-7-8");
        }

        if self.current_metrics.sleep_quality() < 6.0 {
            insights.push("• Поганий сон впливає на все - пріоритизуй 7-8 годин");
        }

        if insights.is_empty() {
            insights.push("• Продовжуй моніторити своє здоров'я регулярно!");
        }

        insights.join("\n")
    }
}

/// Відправити weekly summaries всім користувачам
pub async fn send_weekly_summaries(state: &SharedState) -> Result<()> {
    // Отримати всіх користувачів з Telegram ID
    let users = db::get_all_telegram_users(&state.pool).await?;

    tracing::info!("Sending weekly summaries to {} users", users.len());

    for (user_id, telegram_id) in users {
        match WeeklySummary::generate(&state.pool, user_id).await {
            Ok(summary) => {
                match summary.format_telegram_message(&state.pool, &state.crypto).await {
                    Ok(msg) => {
                        let bot = teloxide::Bot::new(
                            std::env::var("TELEGRAM_BOT_TOKEN")
                                .expect("TELEGRAM_BOT_TOKEN missing"),
                        );

                        if let Err(e) = bot
                            .send_message(ChatId(telegram_id), msg)
                            .parse_mode(ParseMode::Markdown)
                            .await
                        {
                            tracing::error!(
                                "Failed to send weekly summary to user {}: {}",
                                user_id,
                                e
                            );
                        }

                        // Rate limiting - 35ms між повідомленнями
                        tokio::time::sleep(std::time::Duration::from_millis(35)).await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to format summary for user {}: {}", user_id, e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to generate summary for user {}: {}", user_id, e);
            }
        }
    }

    tracing::info!("Weekly summaries sent successfully!");
    Ok(())
}
