-- Import wall of tears data from old system
-- Data extracted from opslab-feedback-production.up.railway.app

-- Insert the 6 feedback posts from old system
-- Note: content is already in Ukrainian, we'll insert as encrypted data
-- Using admin user (work.olegkaminskyi@gmail.com) as the poster

-- Get admin user ID (will be used for all posts)
DO $$
DECLARE
    admin_id UUID;
    enc_key TEXT;
BEGIN
    -- Get admin user
    SELECT id INTO admin_id FROM users WHERE email = 'work.olegkaminskyi@gmail.com';

    IF admin_id IS NULL THEN
        RAISE EXCEPTION 'Admin user not found';
    END IF;

    -- Insert post 1: Negative feedback about hiring
    INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
    VALUES (
        '32a4d32b-0126-4ba7-b924-d1559ff62591'::UUID,
        admin_id,
        'Основна проблема полягає в неефективності процесу найму стажерів, що потребує великих ресурсів, але не приносить очікуваних результатів. Співробітник вказує на необхідність перегляду підходу до найму та адаптації нових людей, оскільки поточна система не здатна забезпечити стабільність і підтримку потрібного ритму роботи.'::BYTEA,
        'COMPLAINT'::post_category,
        TRUE,
        '2025-12-23 14:05:19.76854+00'::TIMESTAMPTZ
    ) ON CONFLICT (id) DO NOTHING;

    -- Insert post 2: Mixed feedback about workload/burnout
    INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
    VALUES (
        '57d8f85e-839e-4fcb-bc25-e1254e063b10'::UUID,
        admin_id,
        'Співробітник високо оцінює культуру зростання в OpsLab, але відчуває труднощі в підтримці постійного високого темпу роботи. Постійний стрес через дедлайни та вимоги швидкості результатів викликають виснаження та брак енергії для креативної роботи.'::BYTEA,
        'SUPPORT_NEEDED'::post_category,
        TRUE,
        '2025-12-23 13:58:13.841826+00'::TIMESTAMPTZ
    ) ON CONFLICT (id) DO NOTHING;

    -- Insert post 3: Mixed feedback about vacation policy
    INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
    VALUES (
        'f5893094-8a11-451f-8f0c-e979e4f988c1'::UUID,
        admin_id,
        'Відгук акцентує на недоліках системи відпусток і лікарняних, зокрема, на відсутності прозорості щодо впровадження нових правил і зменшення кількості лікарняних днів. Автор підкреслює важливість врахування думки команди та турботи про ментальне здоров''я працівників.'::BYTEA,
        'COMPLAINT'::post_category,
        TRUE,
        '2025-12-23 13:46:44.173141+00'::TIMESTAMPTZ
    ) ON CONFLICT (id) DO NOTHING;

    -- Insert post 4: Positive feedback about team growth
    INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
    VALUES (
        '46041b3a-671e-405d-bd1b-35e72da4252c'::UUID,
        admin_id,
        'Співробітник висловлює радість і захоплення від роботи в компанії, особливо підкреслюючи успішний кейс з консалтингу і взаємодії з клієнтом. Він відзначає, що команда дорослішає і розвивається, що викликає у нього гордість.'::BYTEA,
        'CELEBRATION'::post_category,
        TRUE,
        '2025-12-03 18:56:06.161045+00'::TIMESTAMPTZ
    ) ON CONFLICT (id) DO NOTHING;

    -- Insert post 5: Positive feedback (non-anonymous)
    INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
    VALUES (
        '5b913e38-1f22-4aaa-a237-565ba3a0ddec'::UUID,
        admin_id,
        'Автор відзначає задоволення від роботи з командою OpsLab, підкреслюючи акцент на особистісному зростанні та якісній взаємодії з менеджментом. Наголошує на наявності зон для покращення, але позитивно оцінює прогрес і взаємодію з командою.'::BYTEA,
        'CELEBRATION'::post_category,
        TRUE,
        '2025-12-03 11:05:39.843427+00'::TIMESTAMPTZ
    ) ON CONFLICT (id) DO NOTHING;

    -- Insert post 6: Positive feedback (non-anonymous)
    INSERT INTO wall_posts (id, user_id, enc_content, category, ai_categorized, created_at)
    VALUES (
        '9abd63d9-1dd5-4c47-95b1-0198dc4ee73f'::UUID,
        admin_id,
        'Відгук висловлює велике задоволення від роботи в команді OpsLab та взаємодії з менеджментом. Хоча автор відзначає наявність зон для зростання та необхідність систематизації, він підкреслює позитивну динаміку розвитку команди.'::BYTEA,
        'CELEBRATION'::post_category,
        TRUE,
        '2025-12-03 11:05:32.474591+00'::TIMESTAMPTZ
    ) ON CONFLICT (id) DO NOTHING;

    RAISE NOTICE 'Successfully imported 6 wall posts from old system';
END $$;
