-- Test SQL typecasting
SELECT 
    id::varchar,
    amount :: numeric,
    created_at::timestamp,
    data :: jsonb,
    CAST(price AS decimal(10,2)),
    value::integer
FROM users;
