import argparse
import os
import random
from hashlib import sha256

from dotenv import load_dotenv
from faker import Faker
from psycopg_pool import ConnectionPool
from tqdm import tqdm

DEFAULT_USERS = 100_000
DEFAULT_MESSAGES = 100_000

load_dotenv()

DB_HOST = os.getenv("DB_HOST", "0.0.0.0")
DB_PORT = os.getenv("DB_PORT", "5432")
DB_USER = os.getenv("DB_USER", "postgres")
DB_PASSWORD = os.getenv("DB_PASSWORD")
DB_NAME = os.getenv("DB_NAME", "postgres")
DSN = f"postgres://{DB_USER}:{DB_PASSWORD}@{DB_HOST}:{DB_PORT}/{DB_NAME}"

Faker.seed(42)


def insert_users(pool: ConnectionPool, total_users: int) -> None:
    faker = Faker()
    data = [
        (f"user_{i}_{faker.user_name()}", sha256("1234".encode()).hexdigest())
        for i in tqdm(range(total_users), desc="Users")
    ]

    with pool.connection() as conn:
        with conn.cursor() as cur:
            cur.executemany(
                "INSERT INTO users (login, password_hash) VALUES (%s, %s)",
                data,
            )
        conn.commit()


def fetch_user_max_id(pool: ConnectionPool) -> int:
    with pool.connection() as conn:
        with conn.cursor() as cur:
            cur.execute("SELECT COALESCE(MAX(id), 0) FROM users")
            return cur.fetchone()[0]


def insert_messages(pool: ConnectionPool, total_messages: int, user_max_id: int) -> None:
    faker = Faker()
    data = [
        (random.randint(1, user_max_id), faker.sentence(nb_words=12))
        for _ in tqdm(range(total_messages), desc="Messages")
    ]

    with pool.connection() as conn:
        with conn.cursor() as cur:
            cur.executemany(
                "INSERT INTO messages (user_id, text) VALUES (%s, %s)",
                data,
            )
        conn.commit()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-u", "--users", type=int, default=DEFAULT_USERS)
    parser.add_argument("-m", "--messages", type=int, default=DEFAULT_MESSAGES)
    args = parser.parse_args()

    pool = ConnectionPool(conninfo=DSN)

    insert_users(pool, args.users)

    user_max_id = fetch_user_max_id(pool)
    if user_max_id < args.users:
        raise RuntimeError("User insertion incomplete")

    insert_messages(pool, args.messages, user_max_id)

    pool.close()


if __name__ == "__main__":
    main()
