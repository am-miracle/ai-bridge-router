-- Update and expand security audit data with verified information
-- This migration updates existing records and adds new audit reports

-- Update Hop Protocol audits with verified data
DELETE FROM audit_reports WHERE bridge = 'Hop';
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Hop', 'Solidified', '2021-05-05', 'passed');

-- Update Axelar audits with comprehensive verified data
DELETE FROM audit_reports WHERE bridge = 'Axelar';
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Axelar', 'Cure53', '2021-12-01', 'passed'),
    ('Axelar', 'Cure53', '2022-04-01', 'passed'),
    ('Axelar', 'CertiK', '2022-04-01', 'passed'),
    ('Axelar', 'NCC Group', '2022-06-01', 'passed'),
    ('Axelar', 'Ackee Blockchain', '2023-08-01', 'passed'),
    ('Axelar', 'Oak Security', '2024-03-01', 'passed'),
    ('Axelar', 'Code4rena', '2024-08-08', 'passed');

-- Add Wormhole audit history
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Wormhole', 'Neodyme', '2021-08-12', 'passed'),
    ('Wormhole', 'Neodyme', '2022-01-01', 'passed'),
    ('Wormhole', 'Kudelski', '2022-07-01', 'passed'),
    ('Wormhole', 'Kudelski', '2022-08-16', 'passed'),
    ('Wormhole', 'OtterSec', '2022-09-01', 'passed'),
    ('Wormhole', 'OtterSec', '2022-10-08', 'passed'),
    ('Wormhole', 'OtterSec', '2023-02-01', 'passed'),
    ('Wormhole', 'OtterSec', '2023-04-01', 'passed')
ON CONFLICT DO NOTHING;

-- Add Stargate/LayerZero audits
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Stargate', 'Zellic', '2022-03-01', 'issues found'),
    ('Stargate', 'Zellic', '2022-04-01', 'passed'),
    ('Stargate', 'Zellic', '2022-05-01', 'passed'),
    ('Stargate', 'Zellic', '2022-06-01', 'passed'),
    ('Stargate', 'Zellic', '2022-09-01', 'passed'),
    ('Stargate', 'Zellic', '2022-11-01', 'passed'),
    ('Stargate', 'Zellic', '2022-12-01', 'passed'),
    ('Stargate', 'Zellic', '2023-01-01', 'passed')
ON CONFLICT DO NOTHING;

-- Add Across Protocol audits
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Across', 'OpenZeppelin', '2022-07-21', 'passed'),
    ('Across', 'OpenZeppelin', '2025-01-01', 'passed')
ON CONFLICT DO NOTHING;

-- Update Connext to Everclear and add audits
DELETE FROM audit_reports WHERE bridge = 'Connext';
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Everclear', 'Spearbit', '2022-06-01', 'passed'),
    ('Everclear', 'Code4rena', '2022-06-19', 'passed'),
    ('Everclear', 'Macro', '2023-10-13', 'passed');

-- Add LayerZero audits
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('LayerZero', 'Ackee Blockchain', '2022-03-15', 'passed'),
    ('LayerZero', 'Zellic', '2022-04-15', 'passed'),
    ('LayerZero', 'Zellic', '2022-05-21', 'passed'),
    ('LayerZero', 'Zellic', '2022-06-03', 'passed'),
    ('LayerZero', 'ChainSecurity', '2024-06-01', 'passed'),
    ('LayerZero', 'Paladin', '2024-06-06', 'passed')
ON CONFLICT DO NOTHING;

-- Add Orbiter Finance audits
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Orbiter', 'SlowMist', '2023-01-01', 'passed'),
    ('Orbiter', 'SlowMist', '2023-06-01', 'passed'),
    ('Orbiter', 'SlowMist', '2023-09-01', 'passed')
ON CONFLICT DO NOTHING;

-- Add Celer cBridge audits
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('cBridge', 'CertiK', '2021-06-01', 'passed'),
    ('cBridge', 'PeckShield', '2021-09-01', 'passed'),
    ('cBridge', 'PeckShield', '2022-03-01', 'passed'),
    ('cBridge', 'SlowMist', '2022-06-01', 'passed'),
    ('cBridge', 'SlowMist', '2022-09-01', 'passed')
ON CONFLICT DO NOTHING;

-- Add Synapse Protocol audits
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Synapse', 'CertiK', '2021-04-06', 'passed'),
    ('Synapse', 'Quantstamp', '2021-08-01', 'passed'),
    ('Synapse', 'OpenZeppelin', '2021-10-01', 'passed'),
    ('Synapse', 'PeckShield', '2022-01-01', 'passed')
ON CONFLICT DO NOTHING;

-- Add historical bridges for reference
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Nomad', 'Quantstamp', '2022-03-15', 'issues found'),
    ('Multichain', 'PeckShield', '2021-11-20', 'passed')
ON CONFLICT DO NOTHING;

-- Add comprehensive exploit history with verified details
INSERT INTO exploit_history (bridge, incident_date, loss_amount, description)
VALUES
    ('Wormhole', '2022-02-02', 325000000, 'Signature verification bypass leading to unauthorized minting of 120k wETH on Solana. Attacker exploited guardian signature validation flaw. Jump Crypto repaid all funds.'),
    ('Nomad', '2022-08-01', 190000000, 'Replica contract initialization bug allowed arbitrary message execution. Merkle tree root was set to zero during upgrade, allowing anyone to forge messages. Multiple copycats drained the bridge.'),
    ('Ronin', '2022-03-23', 625000000, 'Social engineering attack compromised 5 of 9 validator private keys (Sky Mavis + Axie DAO). Attacker drained 173,600 ETH and 25.5M USDC over multiple transactions.'),
    ('Harmony Horizon', '2022-06-23', 100000000, 'Multi-signature wallet compromise on Ethereum side. 2 of 5 validator keys compromised, allowing attacker to drain bridge funds. Poor key management practices cited as root cause.'),
    ('Poly Network', '2021-08-10', 610000000, 'Cross-chain message exploit via EthCrossChainManager contract. Attacker gained control of privileged functions by exploiting access control flaws. Funds returned after negotiations.'),
    ('BNB Bridge', '2022-10-06', 586000000, 'IAVL proof manipulation in BSC Token Hub. Attacker forged Merkle proofs to mint 2M BNB tokens. Validators halted chain to prevent further damage. Attacker retained ~$100M.'),
    ('Multichain', '2023-07-06', 126000000, 'Suspected team-controlled multisig rug pull. Gradual fund drainage over multiple days from bridge contracts. CEO later arrested by Chinese authorities. Project effectively dead.'),
    ('Qubit Finance', '2022-01-27', 80000000, 'QBridge deposit logic flaw allowed 0 ETH deposits to mint qXETH tokens. Attacker called deposit() with malicious data, bypassing balance checks. All bridge TVL drained in single tx.'),
    ('Chainswap', '2021-07-11', 10000000, 'Multiple exploits targeting cross-chain swap functionality. Private key compromise and smart contract vulnerabilities. Project suffered repeat attacks within weeks.'),
    ('THORChain', '2021-07-15', 8000000, 'Bifrost protocol router exploit. Fake deposit messages bypassed verification, allowing attacker to drain funds. THORChain''s second major exploit within a month.')
ON CONFLICT DO NOTHING;
