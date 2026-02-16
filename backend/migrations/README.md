# Database Migrations

Current migrations:

- `0001_init.up.sql` - initial schema
- `0001_init.down.sql` - rollback of initial schema

## Apply migration manually

```bash
psql "$DATABASE_URL" -f backend/migrations/0001_init.up.sql
```

## Rollback migration manually

```bash
psql "$DATABASE_URL" -f backend/migrations/0001_init.down.sql
```

## Docker-based apply (without local psql)

```bash
cat backend/migrations/0001_init.up.sql | docker compose exec -T postgres psql -U uran -d uran
```

Rollback:

```bash
cat backend/migrations/0001_init.down.sql | docker compose exec -T postgres psql -U uran -d uran
```
