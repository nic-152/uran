# Database Migrations

Current migrations:

- `0001_init.up.sql` - initial schema
- `0001_init.down.sql` - rollback of initial schema
- `0002_controlled_manual_workflow.up.sql` - controlled manual testing workflow schema (versions, runs, results, attachments, audit)
- `0002_controlled_manual_workflow.down.sql` - rollback of migration `0002`

## Apply migrations manually

```bash
psql "$DATABASE_URL" -f backend/migrations/0001_init.up.sql
psql "$DATABASE_URL" -f backend/migrations/0002_controlled_manual_workflow.up.sql
```

## Rollback manually

```bash
psql "$DATABASE_URL" -f backend/migrations/0002_controlled_manual_workflow.down.sql
psql "$DATABASE_URL" -f backend/migrations/0001_init.down.sql
```

## Docker-based apply (without local psql)

```bash
cat backend/migrations/0001_init.up.sql | docker compose exec -T postgres psql -U uran -d uran
cat backend/migrations/0002_controlled_manual_workflow.up.sql | docker compose exec -T postgres psql -U uran -d uran
```

Rollback:

```bash
cat backend/migrations/0002_controlled_manual_workflow.down.sql | docker compose exec -T postgres psql -U uran -d uran
cat backend/migrations/0001_init.down.sql | docker compose exec -T postgres psql -U uran -d uran
```
