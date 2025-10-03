-- Seed audit_reports table with initial data
INSERT INTO audit_reports (bridge, audit_firm, audit_date, result)
VALUES
    ('Connext', 'OpenZeppelin', '2022-05-01', 'passed'),
    ('Connext', 'Sigma Prime', '2023-03-15', 'passed'),
    ('Hop', 'Trail of Bits', '2022-07-15', 'issues found'),
    ('Hop', 'OpenZeppelin', '2023-01-10', 'passed'),
    ('Axelar', 'CertiK', '2023-01-20', 'passed'),
    ('Axelar', 'Quantstamp', '2023-06-30', 'passed'),
    ('Wormhole', 'Neodyme', '2021-08-12', 'passed'),
    ('Nomad', 'Quantstamp', '2022-03-15', 'issues found'),
    ('Multichain', 'PeckShield', '2021-11-20', 'passed'),
    ('Stargate', 'OpenZeppelin', '2022-02-28', 'passed');

-- Seed exploit_history table with known major incidents
INSERT INTO exploit_history (bridge, incident_date, loss_amount, description)
VALUES
    ('Wormhole', '2022-02-02', 325000000, 'Exploit of bridge contract leading to unauthorized minting of 120k wETH'),
    ('Nomad', '2022-08-01', 190000000, 'Unauthorized transfers due to merkle tree initialization bug, led to copy-cat attacks'),
    ('Ronin', '2022-03-23', 625000000, 'Sky Mavis Ronin bridge exploit via compromised validator keys'),
    ('Harmony Horizon', '2022-06-23', 100000000, 'Multisig wallet compromise on Ethereum side of the bridge'),
    ('Poly Network', '2021-08-10', 610000000, 'Cross-chain protocol exploit (funds later returned)'),
    ('BNB Bridge', '2022-10-06', 586000000, 'BSC Token Hub bridge exploit via proof manipulation'),
    ('Multichain', '2023-07-06', 126000000, 'Team-controlled multisig compromise, gradual fund drainage'),
    ('Qubit Finance', '2022-01-27', 80000000, 'Bridge deposit function exploit allowing 0 ETH deposits for qXETH'),
    ('Chainswap', '2021-07-11', 10000000, 'Multiple exploits targeting cross-chain functionality'),
    ('THORChain', '2021-07-15', 8000000, 'Bifrost protocol exploit in cross-chain router');
