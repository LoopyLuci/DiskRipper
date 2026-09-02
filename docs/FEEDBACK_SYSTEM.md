# Feedback Collection System

## Overview

DiskRipper includes a built-in feedback collection system that allows users to report bugs, request features, and share their experience.

## How It Works

### In-App Feedback

1. User clicks **Help → Send Feedback** in the menu
2. A dialog appears with:
   - Feedback type (Bug Report / Feature Request / General)
   - Subject line
   - Description text
   - System info (auto-collected)
   - Screenshot attachment (optional)
3. User submits feedback
4. Feedback is sent to the backend service

### Auto-Collected System Information

The following information is collected automatically (with user consent):

```json
{
  "app_version": "0.1.0",
  "os": "Windows 10",
  "os_version": "10.0.19045",
  "architecture": "x64",
  "drive_info": {
    "model": "HL-DT-ST DVDRAM GH24NS95",
    "firmware": "1.00"
  },
  "settings": {
    "read_speed": null,
    "verify_checksums": true,
    "read_retries": 3
  }
}
```

**No personally identifiable information is collected.**

## Feedback Service

### API Endpoints

```
POST /api/feedback          — Submit feedback
GET  /api/feedback          — List feedback (admin)
GET  /api/feedback/:id      — Get specific feedback
PUT  /api/feedback/:id      — Update status (admin)
DELETE /api/feedback/:id    — Delete feedback (admin)
```

### Feedback Schema

```json
{
  "id": "uuid",
  "type": "bug|feature|general",
  "subject": "String (max 200 chars)",
  "description": "String (max 5000 chars)",
  "system_info": { /* auto-collected */ },
  "screenshot": "base64 encoded image (optional)",
  "email": "optional contact email",
  "status": "new|in_progress|resolved|closed",
  "created_at": "ISO 8601 timestamp",
  "updated_at": "ISO 8601 timestamp"
}
```

### Implementation

The feedback service is implemented as a simple REST API using Actix-web (Rust) or can use a third-party service.

#### Option 1: Self-Hosted (Actix-web)

```rust
// src/feedback.rs
use actix_web::{web, App, HttpServer, HttpResponse, post, get};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Feedback {
    id: String,
    feedback_type: String,
    subject: String,
    description: String,
    system_info: serde_json::Value,
    screenshot: Option<String>,
    email: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

#[post("/api/feedback")]
async fn submit_feedback(feedback: web::Json<FeedbackInput>) -> HttpResponse {
    let feedback = Feedback {
        id: Uuid::new_v4().to_string(),
        feedback_type: feedback.feedback_type.clone(),
        subject: feedback.subject.clone(),
        description: feedback.description.clone(),
        system_info: feedback.system_info.clone(),
        screenshot: feedback.screenshot.clone(),
        email: feedback.email.clone(),
        status: "new".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    // Store in database
    // TODO: Implement storage
    
    HttpResponse::Created().json(feedback)
}

#[get("/api/feedback")]
async fn list_feedback() -> HttpResponse {
    // TODO: Implement listing
    HttpResponse::Ok().json(vec![])
}
```

#### Option 2: Third-Party Services

| Service | Type | Cost | Setup |
|---------|------|------|-------|
| **GitHub Issues** | Bug tracking | Free | Use GitHub API |
| **Sentry** | Error tracking | Free tier | SDK integration |
| **Formspree** | Form handling | Free tier | HTML form |
| **Google Forms** | Surveys | Free | Embed link |
| **Typeform** | Forms | Free tier | Embed link |

### Recommended Approach

For v0.1.0, use **GitHub Issues** via API:

```rust
async fn submit_github_issue(feedback: &Feedback) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let repo = "LoopyLuci/DiskRipper";
    
    let title = format!("[{}] {}", feedback.feedback_type.to_uppercase(), feedback.subject);
    let body = format!(
        "## Description\n{}\n\n## System Info\n```json\n{}\n```\n\n## Feedback ID\n{}",
        feedback.description,
        serde_json::to_string_pretty(&feedback.system_info).unwrap(),
        feedback.id
    );
    
    let issue = serde_json::json!({
        "title": title,
        "body": body,
        "labels": [feedback.feedback_type]
    });
    
    client
        .post(format!("https://api.github.com/repos/{}/issues", repo))
        .header("Authorization", format!("token {}", GITHUB_TOKEN))
        .header("User-Agent", "DiskRipper")
        .json(&issue)
        .send()
        .await?;
    
    Ok(())
}
```

## Privacy Policy

### What We Collect
- App version and OS type
- Drive model and firmware
- User-submitted feedback text
- Optional contact email

### What We DON'T Collect
- Personal files or file contents
- IP addresses (unless required by law)
- Usage patterns or analytics (without consent)
- Disc content or metadata

### Data Storage
- Feedback is stored securely
- Data is not shared with third parties
- Users can request deletion of their feedback

## User Interface

### Feedback Dialog

```
┌─────────────────────────────────────────┐
│  Send Feedback                          │
├─────────────────────────────────────────┤
│  Type: [Bug Report ▼]                   │
│                                         │
│  Subject: [________________]            │
│                                         │
│  Description:                           │
│  ┌─────────────────────────────────┐    │
│  │                                 │    │
│  │                                 │    │
│  └─────────────────────────────────┘    │
│                                         │
│  [✓] Include system information         │
│  [✓] Include log files                  │
│                                         │
│  Email (optional): [______________]     │
│                                         │
│  [Attach Screenshot]                    │
│                                         │
│  [Cancel]              [Submit]         │
└─────────────────────────────────────────┘
```

### Settings

In Settings → Privacy:
- [ ] Allow anonymous usage statistics
- [ ] Allow crash reporting
- [ ] Include system info in feedback (default: on)

## Implementation Checklist

- [ ] Create feedback dialog UI
- [ ] Implement system info collection
- [ ] Set up GitHub Issues integration
- [ ] Add screenshot attachment
- [ ] Add privacy settings
- [ ] Write privacy policy
- [ ] Test feedback submission
- [ ] Add feedback menu item
- [ ] Add keyboard shortcut (Ctrl+Shift+F)
