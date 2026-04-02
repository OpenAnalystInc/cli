#!/bin/bash
set -euo pipefail
exec > /var/log/openanalyst-setup.log 2>&1

echo "=== OpenAnalyst KB Setup Starting ==="

# ── System packages ──────────────────────────────────────────────────
apt-get update -y
apt-get install -y python3.12 python3.12-venv python3-pip git curl unzip apt-transport-https

# ── Install Neo4j Community 5.x ─────────────────────────────────────
curl -fsSL https://debian.neo4j.com/neotechnology.gpg.key | gpg --dearmor -o /usr/share/keyrings/neo4j.gpg
echo "deb [signed-by=/usr/share/keyrings/neo4j.gpg] https://debian.neo4j.com stable latest" > /etc/apt/sources.list.d/neo4j.list
apt-get update -y
apt-get install -y neo4j

# Configure Neo4j
NEO4J_PASS="openanalyst-kb-2026"
neo4j-admin dbms set-initial-password "$NEO4J_PASS"

# Allow remote connections
sed -i 's/#server.default_listen_address=0.0.0.0/server.default_listen_address=0.0.0.0/' /etc/neo4j/neo4j.conf

# Enable vector index support
echo "db.tx_log.rotation.retention_policy=1 days" >> /etc/neo4j/neo4j.conf

# Start Neo4j
systemctl enable neo4j
systemctl start neo4j

# ── Deploy backend ───────────────────────────────────────────────────
mkdir -p /opt/openanalyst-kb
cd /opt/openanalyst-kb

# Clone the repo
git clone https://github.com/AnitChaudhry/openanalyst-cli.git repo
cp -r repo/backend/* .

# Create Python venv
python3.12 -m venv venv
source venv/bin/activate

# Install dependencies
pip install --no-cache-dir -r requirements.txt

# Create .env
cat > .env << 'ENVEOF'
HOST=0.0.0.0
PORT=8420

# Auth
OPENANALYST_API_KEYS=oa_dev_key_openanalyst_2026

# Backend
KB_BACKEND=neo4j

# Neo4j
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=openanalyst-kb-2026
NEO4J_DATABASE=neo4j

# Embeddings
EMBEDDING_MODEL=BAAI/bge-small-en-v1.5

# Synthesis (set your key)
SYNTHESIS_PROVIDER=gemini
GEMINI_API_KEY=
OPENAI_API_KEY=
ANTHROPIC_API_KEY=
ENVEOF

# ── Create systemd service ──────────────────────────────────────────
cat > /etc/systemd/system/openanalyst-kb.service << 'SVCEOF'
[Unit]
Description=OpenAnalyst Knowledge Base API
After=network.target neo4j.service
Wants=neo4j.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/openanalyst-kb
Environment=PATH=/opt/openanalyst-kb/venv/bin:/usr/bin:/bin
ExecStart=/opt/openanalyst-kb/venv/bin/uvicorn app:app --host 0.0.0.0 --port 8420
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

systemctl daemon-reload
systemctl enable openanalyst-kb
systemctl start openanalyst-kb

echo "=== OpenAnalyst KB Setup Complete ==="
echo "Neo4j:  bolt://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):7687"
echo "API:    http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):8420"
echo "Neo4j Browser: http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):7474"
