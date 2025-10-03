-- Create audit_reports table for storing bridge security audit information
CREATE TABLE audit_reports (
    id SERIAL PRIMARY KEY,
    bridge TEXT NOT NULL,
    audit_firm TEXT NOT NULL,
    audit_date DATE NOT NULL,
    result TEXT NOT NULL, -- e.g. "passed", "issues found"
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create an index on bridge for faster lookups
CREATE INDEX idx_audit_reports_bridge ON audit_reports(bridge);

-- Create an index on audit_date for temporal queries
CREATE INDEX idx_audit_reports_date ON audit_reports(audit_date);
