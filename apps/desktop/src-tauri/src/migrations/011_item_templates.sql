ALTER TABLE flow_items ADD COLUMN template_id TEXT;
UPDATE flow_items
SET template_id = (SELECT template_id FROM flows WHERE flows.id = flow_items.flow_id)
WHERE COALESCE(template_id, '') = '';
