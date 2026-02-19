-- Data maturity improvements for reliable metrics assessment
-- Adds data sufficiency validation to metrics calculation

-- Recreate calculate_user_metrics with data maturity awareness
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
    v_total_answers INTEGER;
    v_unique_days INTEGER;
    v_unique_types INTEGER;
    v_confidence NUMERIC;
    v_sufficient BOOLEAN;
    v_risk_level TEXT;
BEGIN
    -- Count data maturity indicators
    SELECT COUNT(*), COUNT(DISTINCT DATE(created_at)), COUNT(DISTINCT question_type)
    INTO v_total_answers, v_unique_days, v_unique_types
    FROM checkin_answers
    WHERE user_id = p_user_id
      AND created_at >= NOW() - INTERVAL '14 days';

    -- Not enough data at all
    IF v_total_answers < 3 THEN
        RETURN NULL;
    END IF;

    -- Calculate confidence: types (40%) + days (35%) + answers (25%)
    v_confidence := LEAST(1.0,
        (LEAST(v_unique_types::NUMERIC / 5.0, 1.0) * 0.4) +
        (LEAST(v_unique_days::NUMERIC / 5.0, 1.0) * 0.35) +
        (LEAST(v_total_answers::NUMERIC / 21.0, 1.0) * 0.25)
    );

    -- Sufficient for risk: 5+ days, 5+ types, 21+ answers, 0.6+ confidence
    v_sufficient := v_unique_days >= 5 AND v_unique_types >= 5 AND v_total_answers >= 21 AND v_confidence >= 0.6;

    -- Calculate averages for each question type (last 14 days)
    SELECT
        COALESCE(AVG(CASE WHEN question_type = 'mood' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'energy' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'stress' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'sleep' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'workload' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'motivation' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'focus' THEN value END), 5.0),
        COALESCE(AVG(CASE WHEN question_type = 'wellbeing' THEN value END), 5.0)
    INTO
        v_mood_avg, v_energy_avg, v_stress_avg, v_sleep_avg,
        v_workload_avg, v_motivation_avg, v_focus_avg, v_wellbeing_avg
    FROM checkin_answers
    WHERE user_id = p_user_id
      AND created_at >= NOW() - INTERVAL '14 days';

    -- WHO-5 Well-Being Index (0-100)
    v_who5 := LEAST(100, GREATEST(0,
        ((v_mood_avg + v_energy_avg + v_wellbeing_avg) / 3.0 * 10.0)::INTEGER
    ));

    -- PHQ-9 Depression Scale (0-27)
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

    -- Risk level only when data is sufficient
    IF v_sufficient THEN
        v_risk_level := CASE
            WHEN v_who5 < 50 OR v_phq9 >= 15 OR v_gad7 >= 15 OR v_mbi >= 70.0 THEN 'critical'
            WHEN v_who5 < 60 OR v_phq9 >= 10 OR v_gad7 >= 10 OR v_mbi >= 50.0 THEN 'high'
            WHEN v_who5 < 70 OR v_phq9 >= 5 OR v_gad7 >= 5 OR v_mbi >= 35.0 THEN 'medium'
            ELSE 'low'
        END;
    ELSE
        v_risk_level := 'collecting_data';
    END IF;

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
            v_sufficient AND (
                v_who5 < 50 OR
                v_phq9 >= 15 OR
                v_gad7 >= 15 OR
                v_mbi >= 70.0
            )
        ),
        'risk_level', v_risk_level,
        'data_sufficient', v_sufficient,
        'data_confidence', ROUND(v_confidence, 2),
        'total_answers', v_total_answers,
        'unique_days', v_unique_days,
        'unique_types', v_unique_types
    );

    RETURN v_metrics;
END;
$$ LANGUAGE plpgsql;
