"""
Копирует задачи одного source-проекта в таблицу tasks
для тестирования production flow.
"""
import sys
from sqlalchemy import text
from config import load_config
from storage import get_engine


# Какой проект использовать как тестовый
PROJECT = sys.argv[1] if len(sys.argv) > 1 else "Apache"

# ID пользователя-админа (который "создал" задачи)
ADMIN_USER_ID = 1


def main():
    config = load_config()
    engine = get_engine(config)

    with engine.begin() as conn:
        # Проверяем что проект существует
        count = conn.execute(
            text("SELECT COUNT(*) FROM ml_source_tasks WHERE source_project_name = :p"),
            {"p": PROJECT},
        ).scalar()

        if count == 0:
            print(f"Project '{PROJECT}' not found in ml_source_tasks")
            print("Available projects:")
            rows = conn.execute(
                text("SELECT DISTINCT source_project_name, COUNT(*) as cnt FROM ml_source_tasks GROUP BY source_project_name")
            ).fetchall()
            for row in rows:
                print(f"  {row[0]}: {row[1]} tasks")
            return

        # Удаляем старые тестовые задачи (если запускали раньше)
        deleted = conn.execute(
            text("DELETE FROM tasks WHERE title LIKE '[TEST]%'")
        ).rowcount
        if deleted > 0:
            print(f"Deleted {deleted} old test tasks")

        # Копируем задачи
        conn.execute(
            text("""
                INSERT INTO tasks
                    (title, description, status, actual_spent_seconds,
                     predicted_seconds, assignee_id, created_by,
                     created_at, updated_at, completed_at)
                SELECT
                    '[TEST] ' || summary,
                    description,
                    'done',
                    actual_spent_seconds,
                    NULL,
                    NULL,
                    :admin_id,
                    COALESCE(completed_at, CURRENT_TIMESTAMP),
                    COALESCE(completed_at, CURRENT_TIMESTAMP),
                    COALESCE(completed_at, CURRENT_TIMESTAMP)
                FROM ml_source_tasks
                WHERE source_project_name = :project
                  AND actual_spent_seconds > 0
            """),
            {"project": PROJECT, "admin_id": ADMIN_USER_ID},
        )

        inserted = conn.execute(
            text("SELECT COUNT(*) FROM tasks WHERE title LIKE '[TEST]%'")
        ).scalar()

        print(f"Inserted {inserted} test tasks from project '{PROJECT}'")

        # Исключаем этот проект из source-корпуса
        # (помечаем другой версией, чтобы не удалять)
        conn.execute(
            text("""
                UPDATE ml_source_tasks
                SET dataset_version = 'v1_excluded'
                WHERE source_project_name = :project
                  AND dataset_version = 'v1'
            """),
            {"project": PROJECT},
        )
        print(f"Excluded '{PROJECT}' from source corpus (version -> v1_excluded)")

        # Итого
        remaining = conn.execute(
            text("SELECT COUNT(*) FROM ml_source_tasks WHERE dataset_version = 'v1'")
        ).scalar()
        print(f"Remaining source tasks: {remaining}")


if __name__ == "__main__":
    main()