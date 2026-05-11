#[cfg(test)]
mod auth_tests {
    use crate::auth;

    #[test]
    fn test_hash_and_verify_password_correct() {
        let password = "super_secret_123";
        let hash = auth::hash_password(password).expect("should hash");
        let result = auth::verify_password(password, &hash).expect("should verify");
        assert!(result);
    }

    #[test]
    fn test_verify_wrong_password() {
        let hash = auth::hash_password("correct_password").expect("should hash");
        let result = auth::verify_password("wrong_password", &hash).expect("should verify");
        assert!(!result);
    }

    #[test]
    fn test_create_and_verify_token() {
        let secret = "test_secret";
        let token = auth::create_token(42, "admin", secret).expect("should create token");
        let claims = auth::verify_token(&token, secret).expect("should verify");
        assert_eq!(claims.sub, 42);
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let token = auth::create_token(1, "member", "real_secret").expect("should create token");
        let result = auth::verify_token(&token, "wrong_secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_tampered_token() {
        let mut token = auth::create_token(1, "member", "secret").expect("should create token");
        token.push_str("tampered");
        let result = auth::verify_token(&token, "secret");
        assert!(result.is_err());
    }
}

// ============================================================

#[cfg(test)]
mod repository_tests {
    use sqlx::SqlitePool;
    use crate::repository;
    use crate::models::{CreateTaskRequest, UpdateTaskRequest};

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory DB");

        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                predicted_seconds REAL,
                actual_spent_seconds REAL,
                assignee_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
                created_by INTEGER NOT NULL REFERENCES users(id),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                is_archived INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    async fn create_test_user(pool: &SqlitePool, email: &str) -> i64 {
        repository::create_user(pool, email, "hash", "Test User", "member")
            .await
            .expect("Failed to create test user")
            .id
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let pool = setup_test_db().await;
        let user = repository::create_user(&pool, "alice@test.com", "hash", "Alice", "member")
            .await
            .unwrap();
        assert_eq!(user.email, "alice@test.com");
        assert_eq!(user.name, "Alice");
        assert_eq!(user.role, "member");
    }

    #[tokio::test]
    async fn test_create_user_duplicate_email() {
        let pool = setup_test_db().await;
        repository::create_user(&pool, "dup@test.com", "hash", "User1", "member")
            .await
            .unwrap();

        let result = repository::create_user(&pool, "dup@test.com", "hash", "User2", "member")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::errors::AppError::BadRequest(msg) => {
                assert!(msg.contains("Email already exists"));
            }
            e => panic!("Expected BadRequest, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_task_with_assignee_and_description() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool, "bob@test.com").await;

        let req = CreateTaskRequest {
            title: "Fix bug".to_string(),
            description: Some("Detailed description".to_string()),
            assignee_id: Some(user_id),
        };

        let task = repository::create_task(&pool, &req, user_id, Some(3600.0))
            .await
            .unwrap();

        assert_eq!(task.title, "Fix bug");
        assert_eq!(task.description, Some("Detailed description".to_string()));
        assert_eq!(task.assignee_id, Some(user_id));
        assert_eq!(task.created_by, user_id);
        assert!((task.predicted_seconds.unwrap() - 3600.0).abs() < 0.01);
        assert_eq!(task.status, "todo");
    }

    #[tokio::test]
    async fn test_get_tasks_by_assignee_returns_correct_tasks() {
        let pool = setup_test_db().await;
        let user1 = create_test_user(&pool, "u1@test.com").await;
        let user2 = create_test_user(&pool, "u2@test.com").await;

        let req1 = CreateTaskRequest {
            title: "Task for user1".to_string(),
            description: None,
            assignee_id: Some(user1),
        };
        let req2 = CreateTaskRequest {
            title: "Task for user2".to_string(),
            description: None,
            assignee_id: Some(user2),
        };

        repository::create_task(&pool, &req1, user1, None).await.unwrap();
        repository::create_task(&pool, &req2, user2, None).await.unwrap();

        let tasks = repository::get_tasks_by_assignee(&pool, user1).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task for user1");
    }

    #[tokio::test]
    async fn test_get_tasks_by_assignee_empty() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool, "empty@test.com").await;

        let tasks = repository::get_tasks_by_assignee(&pool, user_id).await.unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_archive_completed_tasks() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool, "arch@test.com").await;

        let req_done = CreateTaskRequest {
            title: "Done task".to_string(),
            description: None,
            assignee_id: None,
        };
        let req_todo = CreateTaskRequest {
            title: "Todo task".to_string(),
            description: None,
            assignee_id: None,
        };

        let done_task = repository::create_task(&pool, &req_done, user_id, None).await.unwrap();
        repository::create_task(&pool, &req_todo, user_id, None).await.unwrap();

        // Переводим задачу в done
        let update = UpdateTaskRequest {
            title: None,
            description: None,
            status: Some("done".to_string()),
            assignee_id: None,
            actual_hours: None,
        };
        repository::update_task(&pool, done_task.id, &update).await.unwrap();

        let archived = repository::archive_completed_tasks(&pool).await.unwrap();
        assert_eq!(archived, 1);

        // Активных задач должна остаться одна (todo)
        let active = repository::get_all_active_tasks(&pool).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Todo task");
    }

    #[tokio::test]
    async fn test_delete_task_not_found() {
        let pool = setup_test_db().await;
        let result = repository::delete_task(&pool, 99999).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::errors::AppError::NotFound(_) => {}
            e => panic!("Expected NotFound, got {:?}", e),
        }
    }
}

// ============================================================

#[cfg(test)]
mod service_tests {
    use sqlx::SqlitePool;
    use crate::repository;
    use crate::models::{CreateTaskRequest, UpdateTaskRequest};
    use crate::services;
    use crate::errors::AppError;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory DB");

        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                predicted_seconds REAL,
                actual_spent_seconds REAL,
                assignee_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
                created_by INTEGER NOT NULL REFERENCES users(id),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                is_archived INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    async fn create_test_user(pool: &SqlitePool, email: &str) -> i64 {
        repository::create_user(pool, email, "hash", "Test User", "member")
            .await
            .unwrap()
            .id
    }

    async fn create_test_task(pool: &SqlitePool, user_id: i64) -> i64 {
        let req = CreateTaskRequest {
            title: "Test task".to_string(),
            description: None,
            assignee_id: None,
        };
        repository::create_task(pool, &req, user_id, None)
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn test_update_task_invalid_status() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool, "svc@test.com").await;
        let task_id = create_test_task(&pool, user_id).await;

        let req = UpdateTaskRequest {
            title: None,
            description: None,
            status: Some("flying".to_string()),
            assignee_id: None,
            actual_hours: None,
        };

        // Создаём фиктивный MlClient
        // (недоступный сервис — predict_time_safe вернёт None)
        let ml_client = crate::ml_client::MlClient::new("http://localhost:0".to_string());

        let result = services::update_task(&pool, &ml_client, task_id, req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("Status must be"));
            }
            e => panic!("Expected BadRequest, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_update_task_valid_status_transitions() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool, "svc2@test.com").await;
        let task_id = create_test_task(&pool, user_id).await;
        let ml_client = crate::ml_client::MlClient::new("http://localhost:0".to_string());

        for status in ["in_progress", "done", "todo"] {
            let req = UpdateTaskRequest {
                title: None,
                description: None,
                status: Some(status.to_string()),
                assignee_id: None,
                actual_hours: None,
            };
            let result = services::update_task(&pool, &ml_client, task_id, req).await;
            assert!(result.is_ok(), "Transition to '{}' should succeed", status);
        }
    }
}

// ============================================================

#[cfg(test)]
mod handler_tests {
    use actix_web::{test, web, App};
    use sqlx::SqlitePool;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;
    use crate::{handlers, auth, repository};
    use crate::ml_client::MlClient;
    use crate::config::Config;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory DB");

        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                predicted_seconds REAL,
                actual_spent_seconds REAL,
                assignee_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
                created_by INTEGER NOT NULL REFERENCES users(id),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                is_archived INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    fn test_config(ml_url: &str) -> Config {
        Config {
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test_secret".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            ml_service_url: ml_url.to_string(),
            admin_email: "admin@test.com".to_string(),
            admin_password: "password".to_string(),
        }
    }

    async fn create_test_user(pool: &SqlitePool, email: &str, role: &str) -> (i64, String) {
        let user = repository::create_user(pool, email, "hash", "Test", role)
            .await
            .unwrap();
        let token = auth::create_token(user.id, &user.role, "test_secret").unwrap();
        (user.id, token)
    }

    // ---- GET /api/tasks/my без токена ----

    #[tokio::test]
    async fn test_get_my_tasks_no_token() {
        let pool = setup_test_db().await;
        let config = test_config("http://localhost:0");
        let ml_client = MlClient::new("http://localhost:0".to_string());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/tasks/my")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    // ---- POST /api/tasks с mock ML-сервисом ----

    #[tokio::test]
    async fn test_create_task_with_mocked_ml() {
        let mock_server = MockServer::start().await;

        // Настраиваем mock - ML-сервис отвечает предсказанием
        Mock::given(method("POST"))
            .and(path("/predict"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"predicted_seconds": 7200.0})),
            )
            .mount(&mock_server)
            .await;

        let pool = setup_test_db().await;
        let config = test_config(&mock_server.uri());
        let (_, token) = create_test_user(&pool, "creator@test.com", "member").await;
        let ml_client = MlClient::new(mock_server.uri());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/tasks")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "title": "New task",
                "description": "Some description"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["title"], "New task");
        assert!((body["predicted_hours"].as_f64().unwrap() - 2.0).abs() < 0.01);
    }

    // ---- POST /api/tasks при недоступном ML-сервисе ----

    #[tokio::test]
    async fn test_create_task_ml_unavailable() {
        let pool = setup_test_db().await;
        let config = test_config("http://localhost:0");
        let (_, token) = create_test_user(&pool, "noml@test.com", "member").await;
        let ml_client = MlClient::new("http://localhost:0".to_string());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/tasks")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({"title": "Task without ML"}))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Задача должна создаться успешно без ML
        assert_eq!(resp.status(), 201);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["title"], "Task without ML");
        // Предсказание отсутствует
        assert!(body["predicted_hours"].is_null());
    }

    // ---- DELETE /api/users/{id} от member ----

    #[tokio::test]
    async fn test_delete_user_as_member_is_forbidden() {
        let pool = setup_test_db().await;
        let config = test_config("http://localhost:0");
        let (_, member_token) = create_test_user(&pool, "member@test.com", "member").await;
        let (target_id, _) = create_test_user(&pool, "target@test.com", "member").await;
        let ml_client = MlClient::new("http://localhost:0".to_string());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/users/{}", target_id))
            .insert_header(("Authorization", format!("Bearer {}", member_token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    // ---- DELETE /api/users/{id} — удаление себя ----

    #[tokio::test]
    async fn test_delete_self_is_bad_request() {
        let pool = setup_test_db().await;
        let config = test_config("http://localhost:0");
        let (admin_id, admin_token) = create_test_user(&pool, "admin@test.com", "admin").await;
        let ml_client = MlClient::new("http://localhost:0".to_string());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/api/users/{}", admin_id))
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    // ---- GET /api/admin/ml/status от member ----

    #[tokio::test]
    async fn test_ml_status_as_member_is_forbidden() {
        let pool = setup_test_db().await;
        let config = test_config("http://localhost:0");
        let (_, member_token) = create_test_user(&pool, "mlmember@test.com", "member").await;
        let ml_client = MlClient::new("http://localhost:0".to_string());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/admin/ml/status")
            .insert_header(("Authorization", format!("Bearer {}", member_token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    // ---- GET /api/admin/ml/status от admin с mock ML ----

    #[tokio::test]
    async fn test_ml_status_as_admin() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/status"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "active_model": "scratch_ridge",
                    "model_candidate_id": 1,
                    "activated_at": "2025-02-01T12:00:00",
                    "r2_val": 0.65,
                    "medae_val": 120.0,
                    "mdape_val": 35.0
                })),
            )
            .mount(&mock_server)
            .await;

        let pool = setup_test_db().await;
        let config = test_config(&mock_server.uri());
        let (_, admin_token) = create_test_user(&pool, "adm@test.com", "admin").await;
        let ml_client = MlClient::new(mock_server.uri());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/admin/ml/status")
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["active_model"], "scratch_ridge");
        assert!((body["r2_val"].as_f64().unwrap() - 0.65).abs() < 0.001);
    }

    // ---- POST /api/admin/ml/retrain от member ----

    #[tokio::test]
    async fn test_retrain_as_member_is_forbidden() {
        let pool = setup_test_db().await;
        let config = test_config("http://localhost:0");
        let (_, member_token) = create_test_user(&pool, "retmember@test.com", "member").await;
        let ml_client = MlClient::new("http://localhost:0".to_string());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .app_data(web::Data::new(config))
                .app_data(web::Data::new(ml_client))
                .configure(handlers::configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/admin/ml/retrain")
            .insert_header(("Authorization", format!("Bearer {}", member_token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }
}