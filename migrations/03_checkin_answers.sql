-- Daily check-in answers table
-- Stores responses from daily mental health check-ins
-- Each answer is linked to a user and question type

CREATE TABLE IF NOT EXISTS checkin_answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question_id INTEGER NOT NULL,
    question_type VARCHAR(50) NOT NULL,
    value SMALLINT NOT NULL CHECK (value >= 1 AND value <= 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index for efficient queries by user and date
CREATE INDEX IF NOT EXISTS idx_checkin_answers_user_date
    ON checkin_answers(user_id, created_at DESC);

-- Index for querying by question type
CREATE INDEX IF NOT EXISTS idx_checkin_answers_type
    ON checkin_answers(question_type, created_at DESC);

-- Function to get recent checkin answers for a user
CREATE OR REPLACE FUNCTION get_recent_checkin_answers(
    p_user_id UUID,
    p_days INTEGER DEFAULT 10
)
RETURNS TABLE (
    question_id INTEGER,
    question_type VARCHAR(50),
    value SMALLINT,
    created_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        ca.question_id,
        ca.question_type,
        ca.value,
        ca.created_at
    FROM checkin_answers ca
    WHERE ca.user_id = p_user_id
      AND ca.created_at >= NOW() - (p_days || ' days')::INTERVAL
    ORDER BY ca.created_at DESC;
END;
$$ LANGUAGE plpgsql;

-- Function to calculate metrics from checkin answers
CREATE OR REPLACE FUNCTION calculate_user_metrics(p_user_id UUID)
RETURNS JSON AS $$
DECLARE
    v_metrics JSON;
    v_mood_avg NUMERIC;
    v_energy_avg NUMERIC;
    v_stress_avg NUMERIC;
    v_sleep_avg NUMERIC;
    v_workload_avg NUMERIC;
    v_motivation_avg NUMERIC;
    v_focus_avg NUMERIC;
    v_wellbeing_avg NUMERIC;
    v_who5 INTEGER;
    v_phq9 INTEGER;
    v_gad7 INTEGER;
    v_mbi NUMERIC;
BEGIN
    -- Calculate averages for each question type (last 10 days)
    SELECT
        COALESCE(AVG(CASE WHEN question_type = 'mood' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'energy' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'stress' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'sleep' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'workload' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'motivation' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'focus' THEN value END), 0),
        COALESCE(AVG(CASE WHEN question_type = 'wellbeing' THEN value END), 0)
    INTO
        v_mood_avg, v_energy_avg, v_stress_avg, v_sleep_avg,
        v_workload_avg, v_motivation_avg, v_focus_avg, v_wellbeing_avg
    FROM checkin_answers
    WHERE user_id = p_user_id
      AND created_at >= NOW() - INTERVAL '10 days';

    -- WHO-5 Well-Being Index (0-100)
    v_who5 := LEAST(100, GREATEST(0,
        ((v_mood_avg + v_energy_avg + v_wellbeing_avg) / 3.0 * 10.0)::INTEGER
    ));

    -- PHQ-9 Depression Scale (0-27) - inverse of positive indicators
    v_phq9 := LEAST(27, GREATEST(0,
        (((10.0 - v_mood_avg) + (10.0 - v_energy_avg) + (10.0 - v_motivation_avg)) / 3.0 * 2.7)::INTEGER
    ));

    -- GAD-7 Anxiety Scale (0-21)
    v_gad7 := LEAST(21, GREATEST(0,
        ((v_stress_avg + (10.0 - v_focus_avg)) / 2.0 * 2.1)::INTEGER
    ));

    -- MBI Burnout Index (0-100%)
    v_mbi := LEAST(100.0, GREATEST(0.0,
        (v_stress_avg + v_workload_avg + (10.0 - v_energy_avg) + (10.0 - v_motivation_avg)) / 4.0 * 10.0
    ));

    -- Build JSON result
    v_metrics := json_build_object(
        'who5_score', v_who5,
        'phq9_score', v_phq9,
        'gad7_score', v_gad7,
        'mbi_score', ROUND(v_mbi, 1),
        'sleep_duration', ROUND(v_sleep_avg, 1),
        'sleep_quality', ROUND(v_sleep_avg, 1),
        'work_life_balance', ROUND(10.0 - v_workload_avg, 1),
        'stress_level', ROUND(v_stress_avg * 4.0, 1),
        'is_critical', (
            v_who5 < 50 OR
            v_phq9 >= 15 OR
            v_gad7 >= 15 OR
            v_mbi >= 70.0
        ),
        'risk_level', CASE
            WHEN v_who5 < 50 OR v_phq9 >= 15 OR v_gad7 >= 15 OR v_mbi >= 70.0 THEN 'critical'
            WHEN v_who5 < 60 OR v_phq9 >= 10 OR v_gad7 >= 10 OR v_mbi >= 50.0 THEN 'high'
            WHEN v_who5 < 70 OR v_phq9 >= 5 OR v_gad7 >= 5 OR v_mbi >= 35.0 THEN 'medium'
            ELSE 'low'
        END
    );

    RETURN v_metrics;
END;
$$ LANGUAGE plpgsql;
