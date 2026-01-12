# Extracted Data from Old OpsLab Systems

## Date: 2026-01-12
## Extracted by: Claude Code

---

## 1. Wall of Tears (Стіна Плачу) Data

### Source
- URL: https://opslab-feedback-production.up.railway.app
- API Base: `/api`
- Authentication: Bearer token (JWT)
- Login endpoint: `POST /api/auth/login` with `{email, password}`

### Extracted Data

#### Total Feedbacks: **6 posts**

#### Available Months: **1 month**
- грудень 2025 (December 2025 / 2025-12)

#### API Endpoints Discovered
- ✅ `POST /api/auth/login` - Authentication
- ✅ `GET /api/stats/available-months` - Get list of months with data
- ✅ `GET /api/feedback` - Get all feedback posts
- ✅ `GET /api/feedback?month=YYYY-MM` - Get feedbacks for specific month
- ❌ `/api/stats` - Returns HTML (frontend SPA)
- ❌ `/api/feedbacks` (plural) - Returns HTML (frontend SPA)

### Data Structure

Each feedback post contains:
```json
{
  "id": "UUID",
  "created_at": "ISO timestamp",
  "is_anonymous": boolean,
  "sentiment": "positive|negative|mixed",
  "summary": "AI-generated summary in Ukrainian",
  "tags": ["array", "of", "extracted", "tags"],
  "work_aspect": "team|management|workload",
  "emotional_intensity": 1-5 integer score,
  "user_name": "User Name" (only if not anonymous)
}
```

### Content Analysis

**Sentiment Distribution:**
- Positive: 3 posts (50%)
- Mixed: 2 posts (33.3%)
- Negative: 1 post (16.7%)

**Work Aspects:**
- team: 3 posts
- management: 2 posts
- workload: 1 post

**Emotional Intensity:**
- Level 3: 3 posts
- Level 4: 3 posts

**Key Themes Identified:**
1. **Hiring challenges** - Difficulty finding and training right interns
2. **Workload & burnout** - High pace, deadline pressure, exhaustion
3. **Vacation policy issues** - Lack of transparency, reduced sick days
4. **Team growth** - Pride in team development and achievements
5. **Personal growth** - Satisfaction with management quality

### Files Created
- `WALL_ALL_FEEDBACKS.json` - All 6 feedback posts with full data
- `wall_months.json` - Available months: [{"label": "грудень 2025", "value": "2025-12"}]
- `api_api_feedback.json` - Same as WALL_ALL_FEEDBACKS.json

---

## 2. TeamPulse System

### Source
- URL: https://teampulse-mindguard-production.up.railway.app
- Status: **Unable to extract** - API structure appears different or non-existent

### Attempted Endpoints
- ❌ `POST /api/auth/login` - 404 Not Found
- ❌ `POST /auth/login` - 404 Not Found
- ❌ `POST /login` - 404 Not Found

### Notes
This system may be:
1. The NEW platform (not old data to extract)
2. Using different authentication method
3. Backend on different domain
4. Not actually deployed or accessible

**User mentioned:** "також викачай всі помісячні дані як тут https://teampulse-mindguard-production.up.railway.app/"

**Recommendation:** Need to clarify if teampulse-mindguard is:
- The CURRENT new platform (where data should be imported TO)
- An OLD platform (where data should be extracted FROM)

---

## 3. Next Steps

### Completed ✅
1. Found and accessed old wall of tears API
2. Extracted all 6 feedback posts
3. Extracted available months list
4. Documented data structure

### To Do 📋
1. **Create identical UI** - Build exact replica of https://opslab-feedback-production.up.railway.app/stats interface
2. **Import data** - Migrate 6 feedbacks into new platform database
3. **Clarify TeamPulse** - Determine if it's old system to extract or new system to import into
4. **Match interface exactly** - Colors, fonts, layout, interactions
5. **Deploy** - Ensure everything works on backend-production-e745.up.railway.app

---

## 4. Wall of Tears UI Requirements

Based on the URL `/stats` and the frontend being a Vite SPA with:
- Fonts: Space Mono (400, 700) + Space Grotesk (400, 500, 700)
- Language: Ukrainian
- Title: "Стіна Плачу OpsLab | Зворотний зв'язок команди"

The interface should display:
- List of all feedbacks
- Filter by month
- Show sentiment indicators
- Display tags
- Show work aspects
- Emotional intensity visualization
- Anonymous indicator
- Timestamps

### Design System
- Fonts from Google Fonts
- Neobrutal design (based on existing platform design)
- Bold colors and shadows
- Ukrainian language throughout

---

## Files Location
All extracted data is in:
```
/Users/olehkaminskyi/Desktop/Платформа OpsLab Mindguard/
├── WALL_ALL_FEEDBACKS.json
├── wall_months.json
├── api_api_feedback.json
└── EXTRACTED_DATA_SUMMARY.md (this file)
```
