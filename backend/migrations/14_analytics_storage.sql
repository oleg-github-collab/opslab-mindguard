-- Analytics storage for monthly metrics and leadership insights

CREATE TABLE IF NOT EXISTS analytics_monthly_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period DATE NOT NULL,
    who5 DOUBLE PRECISION NOT NULL,
    phq9 DOUBLE PRECISION NOT NULL,
    gad7 DOUBLE PRECISION NOT NULL,
    mbi DOUBLE PRECISION NOT NULL,
    sleep_duration DOUBLE PRECISION NOT NULL,
    sleep_quality DOUBLE PRECISION NOT NULL,
    work_life_balance DOUBLE PRECISION NOT NULL,
    stress_level DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL DEFAULT 'computed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, period)
);

CREATE INDEX IF NOT EXISTS idx_analytics_monthly_metrics_period
    ON analytics_monthly_metrics(period);

CREATE TABLE IF NOT EXISTS analytics_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company TEXT NOT NULL DEFAULT 'OpsLab',
    data_collection_period TEXT,
    update_frequency TEXT,
    next_assessment DATE,
    participation_rate TEXT,
    assessment_tools JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (company)
);

CREATE TABLE IF NOT EXISTS analytics_industry_benchmarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sector TEXT NOT NULL UNIQUE,
    who5 DOUBLE PRECISION,
    phq9 DOUBLE PRECISION,
    gad7 DOUBLE PRECISION,
    mbi DOUBLE PRECISION,
    sleep_duration DOUBLE PRECISION,
    work_life_balance DOUBLE PRECISION,
    stress_level DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS analytics_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    severity TEXT NOT NULL,
    employee_id UUID REFERENCES users(id) ON DELETE SET NULL,
    employee_name TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS analytics_recommendations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    affected_employees JSONB NOT NULL DEFAULT '[]'::jsonb,
    priority TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO analytics_industry_benchmarks (
    sector,
    who5,
    phq9,
    gad7,
    mbi,
    sleep_duration,
    work_life_balance,
    stress_level
) VALUES (
    'tech',
    58.0,
    8.0,
    7.0,
    30.0,
    6.5,
    5.5,
    7.0
)
ON CONFLICT (sector) DO NOTHING;

INSERT INTO analytics_metadata (
    company,
    data_collection_period,
    update_frequency,
    next_assessment,
    participation_rate,
    assessment_tools
) VALUES (
    'OpsLab',
    'Серпень-Грудень 2025',
    'monthly',
    '2026-01-30',
    '100%',
    '["WHO-5 Well-Being Index (0-100)", "PHQ-9 Patient Health Questionnaire (0-27)", "GAD-7 Generalized Anxiety Disorder (0-21)", "MBI Maslach Burnout Inventory (0-100%)"]'::jsonb
)
ON CONFLICT (company) DO NOTHING;
