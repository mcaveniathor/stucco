CREATE INDEX orders_tenant_id_desc ON orders (tenant_id, id DESC);
CREATE INDEX orders_tenant_customer_search ON orders (tenant_id, customer_search, id DESC);
CREATE INDEX users_tenant_id ON users (tenant_id, id);
