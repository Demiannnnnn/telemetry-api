# Legal Compliance — Data Protection Framework

## Applicable Legislation

This document outlines the legal compliance framework for the Telemetry API with respect to data protection laws applicable in Chile and international best practices.

---

## 1. Chilean Data Protection Laws

### 1.1 Ley 19.628 — Protección de la Vida Privada (1999)

**Full name:** Ley sobre Protección de la Vida Privada (Law on the Protection of Private Life)

**Enacted:** August 28, 1999

**Key provisions relevant to this platform:**

| Article | Provision | Compliance Measure |
|---------|-----------|-------------------|
| Art. 4 | Personal data processing requires consent of the data subject | Workers must provide explicit consent before any telemetry collection begins |
| Art. 6 | Data must be collected for the purposes communicated to the data subject | Telemetry categories and their purposes are disclosed at consent time |
| Art. 7 | Data subjects have the right to be informed about the data stored about them | Workers can request an export of all their stored telemetry data |
| Art. 9 | Personal data must be accurate and updated when necessary | Timestamps ensure data integrity; workers can request corrections |
| Art. 10 | Data processing must not affect the data subject's fundamental rights | E2E encryption ensures even the platform operator cannot misuse data |
| Art. 12 | Right to access personal data | API provides data export endpoint for workers |
| Art. 12 | Right to deletion of personal data | API provides data deletion request endpoint |

### 1.2 Ley 21.719 — Ley de Protección de Datos Personales (2024)

**Full name:** Ley que Regula la Protección y el Tratamiento de los Datos Personales (Law Regulating the Protection and Processing of Personal Data)

This law significantly modernizes Chile's data protection framework, aligning it more closely with the European GDPR. It establishes the Agencia de Protección de Datos Personales (Personal Data Protection Agency) as the supervisory authority.

**Key provisions and compliance measures:**

#### 1.2.1 Lawful Bases for Processing (Art. 13)

| Lawful Basis | Application |
|-------------|-------------|
| **Consent** | Primary basis — workers explicitly consent to each telemetry category |
| **Legitimate Interest** | Secondary basis — employer's legitimate interest in IT security and productivity monitoring on corporate devices |

> **Note:** Even when legitimate interest could apply, we choose to obtain explicit consent for each telemetry category as a stronger compliance position.

#### 1.2.2 Principles of Data Processing

| Principle | Implementation |
|-----------|---------------|
| **Lawfulness** (Art. 3 a) | Processing based on explicit, informed consent |
| **Purpose Limitation** (Art. 3 b) | Data used exclusively for stated telemetry and monitoring purposes |
| **Proportionality** (Art. 3 c) | Only minimum necessary data points collected per category |
| **Quality** (Art. 3 d) | Timestamps ensure accuracy; stale data is purged per retention policy |
| **Accountability** (Art. 3 e) | Audit logs track all data access and processing operations |
| **Security** (Art. 3 f) | E2E encryption, JWT auth, TLS, input validation |
| **Transparency** (Art. 3 g) | Clear disclosure of what data is collected, how, and why |

#### 1.2.3 Data Subject Rights (Title III)

| Right | Article | Implementation |
|-------|---------|----------------|
| **Right of Access** | Art. 14 | `GET /workers/:id` and data export endpoint |
| **Right of Rectification** | Art. 15 | Workers can update their profile data |
| **Right of Deletion** | Art. 16 | `POST /workers/me/data-deletion` endpoint |
| **Right of Opposition** | Art. 17 | Workers can revoke consent for any category at any time |
| **Right of Portability** | Art. 18 | `POST /workers/me/data-request` provides machine-readable data export |
| **Right to Withdraw Consent** | Art. 13 | `PATCH /workers/:id/consent` endpoint; revocation stops future collection immediately |

> **Important:** When a worker revokes consent for a category, data collection for that category MUST stop immediately. Previously collected data remains until explicitly deleted.

#### 1.2.4 Data Breach Notification (Art. 50)

In the event of a personal data breach:

1. **Notify the Agencia de Protección de Datos Personales** within **72 hours** of becoming aware of the breach
2. **Notify affected data subjects** without undue delay if the breach is likely to result in high risk to their rights
3. **Documentation:** Maintain records of all breaches regardless of notification obligation
4. **Content of notification:**
   - Nature of the breach
   - Categories and approximate number of data subjects affected
   - Likely consequences
   - Measures taken or proposed to address the breach

#### 1.2.5 Data Protection Impact Assessment (EIPD)

Under Art. 49, a Data Protection Impact Assessment (Evaluación de Impacto en la Protección de Datos, EIPD) is required when processing is likely to result in high risk to data subjects. This platform requires an EIPD because:

- It processes employee monitoring data (inherently high-risk)
- It involves systematic observation of data subjects
- It processes location data (sensitive category)
- It captures screenshots (significant privacy impact)

See Section 5 below for the EIPD summary.

---

## 2. International References

### 2.1 GDPR Alignment

While not directly applicable in Chile, the Ley 21.719 is modeled after the EU GDPR. This platform's compliance measures are designed to meet GDPR-equivalent standards:

| GDPR Principle | Platform Compliance |
|---------------|-------------------|
| Data Protection by Design (Art. 25) | E2E encryption as a core architectural decision |
| Data Protection by Default (Art. 25) | All consent flags default to `false`; opt-in required |
| Records of Processing (Art. 30) | Audit log system tracks all data operations |
| Data Minimization (Art. 5) | Only necessary telemetry data points collected |
| Storage Limitation (Art. 5) | Configurable retention periods with automatic cleanup |
| Integrity and Confidentiality (Art. 5) | E2E encryption, access controls, audit trails |

### 2.2 ILO Guidelines on Workers' Privacy

The International Labour Organization (ILO) Code of Practice on the Protection of Workers' Personal Data provides additional guidance:

| Guideline | Compliance |
|-----------|-----------|
| Monitoring should be proportionate | Only necessary categories collected; per-category consent |
| Workers should be informed | Full transparency on what is collected and why |
| Personal data should be protected | E2E encryption; zero-knowledge server |
| Workers should have access to their data | Data export and access endpoints provided |
| Monitoring should not be used for disciplinary purposes without notice | Platform provides data, not disciplinary recommendations |

---

## 3. Consent Framework

### 3.1 Consent Requirements

For consent to be legally valid under Chilean law:

| Requirement | Implementation |
|------------|----------------|
| **Free** | Worker can refuse or withdraw consent at any time without consequences |
| **Specific** | Consent is granular per telemetry category (6 independent flags) |
| **Informed** | Clear description of each data category, purpose, and retention period |
| **Unambiguous** | Explicit opt-in via `PATCH /workers/:id/consent` (no pre-checked boxes) |
| **Documented** | Consent changes are recorded in the audit log with timestamps |
| **Revocable** | Consent can be withdrawn at any time; collection stops immediately |

### 3.2 Consent Categories

| Category | Consent Flag | Sensitivity | Notice Required |
|----------|-------------|-------------|----------------|
| System Activity | `system_activity` | Medium | Standard |
| Network Activity | `network_activity` | Medium | Standard |
| Productivity (Basic) | `productivity_basic` | Medium | Standard |
| Productivity (Screenshots) | `productivity_screenshots` | High | Visible notification at capture time |
| File Activity | `file_activity` | Medium | Standard |
| Location (GPS) | `location` | Very High | Visible indicator when tracking active |

### 3.3 Enhanced Consent for Sensitive Categories

**Screenshots** and **Location** require enhanced consent procedures:

1. Separate, distinct consent (not bundled with other categories)
2. Clear explanation of the specific data collected
3. Information about frequency of collection
4. Visible notification during active collection
5. Easy one-step revocation
6. Additional operational justification documented by the admin

---

## 4. Data Processing Activities Register

Per Art. 34 of Ley 21.719, the following register documents all processing activities:

| Activity | Purpose | Lawful Basis | Categories | Recipients | Retention | Transfer |
|----------|---------|-------------|-----------|-----------|-----------|----------|
| Worker Registration | Employee monitoring setup | Consent + Legitimate Interest | Name, email, device ID, role | Admin only | Until account deletion | None |
| System Activity Collection | IT performance monitoring | Consent | Idle time, apps, windows, resources, USB | Admin (encrypted) | 90 days (configurable) | None |
| Network Activity Collection | Security compliance | Consent | Domains, categories | Admin (encrypted) | 90 days (configurable) | None |
| Productivity Collection | Activity measurement | Consent | Keystroke count, click count | Admin (encrypted) | 90 days (configurable) | None |
| Screenshot Collection | Visual verification | Explicit Consent | Screen captures | Admin (encrypted) | 30 days (configurable) | None |
| File Activity Collection | Data security | Consent | File access logs | Admin (encrypted) | 90 days (configurable) | None |
| Location Collection | Field worker tracking | Explicit Consent | GPS coordinates | Admin (encrypted) | 30 days (configurable) | None |
| Authentication | Access control | Legitimate Interest | Email, password hash, tokens | System only | Until account deletion | None |
| Audit Logging | Compliance | Legal Obligation | User actions, IP, user agent | System only | 2 years | None |

---

## 5. Data Protection Impact Assessment (EIPD) — Summary

### 5.1 Processing Description

| Aspect | Description |
|--------|-------------|
| **Nature** | Systematic monitoring of employee device activity |
| **Scope** | All workers who consent, during work hours |
| **Context** | Corporate devices, employment relationship |
| **Purpose** | Productivity analysis, IT security, operational efficiency |

### 5.2 Necessity and Proportionality Assessment

| Question | Assessment |
|----------|-----------|
| Is the processing necessary for the purpose? | Yes — telemetry requires data collection |
| Are there less intrusive alternatives? | Self-reported metrics, but these are unreliable for the stated purpose |
| Is the data collection proportionate? | Yes — only frequency metrics, not content; granular consent |
| Is the purpose clearly defined? | Yes — productivity monitoring and IT security |

### 5.3 Risk Assessment

| Risk | Likelihood | Severity | Mitigation | Residual Risk |
|------|-----------|----------|------------|---------------|
| Unauthorized data access | Low | High | E2E encryption, RBAC, ownership checks | Very Low |
| Data breach exposing telemetry | Low | High | Data encrypted at rest; breach yields only ciphertext | Very Low |
| Excessive surveillance | Medium | High | Granular consent, purpose limitation, data minimization | Low |
| Function creep (data used for unintended purposes) | Medium | Medium | Purpose limitation, audit logs, legal contractual obligations | Low |
| Chilling effect on workers | Medium | Medium | Transparency, visible notifications, easy consent revocation | Low |
| GPS tracking outside work hours | Low | Very High | Server-side work hours validation, auto-stop | Very Low |
| Screenshot privacy violations | Medium | High | Separate explicit consent, visible notification, low frequency | Low |
| Re-identification from metadata | Low | Medium | Minimal metadata stored; encrypted payloads | Very Low |

### 5.4 Measures to Mitigate Risk

1. **End-to-end encryption** — Server cannot access plaintext data
2. **Granular consent** — Workers control each telemetry category independently
3. **Visible notifications** — Workers know when screenshots/GPS are active
4. **Data minimization** — Only frequency metrics, never content
5. **Retention limits** — Automatic data deletion after retention period
6. **Audit trail** — All data access operations logged
7. **Data subject rights** — Export, deletion, and opposition endpoints available
8. **Work hours enforcement** — GPS tracking auto-stops outside configured hours
9. **Role-based access** — Admins only see their own workers' data

---

## 6. Data Retention Policy

| Data Category | Default Retention | Minimum | Maximum | After Expiry |
|---------------|-------------------|---------|---------|--------------|
| Admin Accounts | Indefinite | - | - | Soft delete |
| Worker Accounts | Indefinite | - | - | Soft delete |
| System Activity | 90 days | 30 days | 365 days | Hard delete |
| Network Activity | 90 days | 30 days | 365 days | Hard delete |
| Productivity (Basic) | 90 days | 30 days | 365 days | Hard delete |
| Productivity (Screenshots) | 30 days | 7 days | 90 days | Hard delete |
| Location Data | 30 days | 7 days | 90 days | Hard delete |
| File Activity | 90 days | 30 days | 365 days | Hard delete |
| Refresh Tokens | 30 days | - | - | Hard delete |
| Audit Logs | 2 years | 1 year | 5 years | Archive |

---

## 7. Contractual Obligations

### 7.1 Terms of Service

The admin (as data controller) must agree to:

1. Using the platform only for legitimate business purposes
2. Informing workers about the monitoring before activation
3. Not using telemetry data for discriminatory purposes
4. Responding to worker data subject requests within legal timeframes
5. Maintaining the security of their E2E encryption keys
6. Complying with all applicable data protection laws

### 7.2 Data Processing Agreement (DPA)

As the platform operator (data processor), a DPA should cover:

1. Subject matter and duration of processing
2. Nature and purpose of processing
3. Type of personal data processed
4. Categories of data subjects
5. Obligations and rights of the controller
6. Technical and organizational security measures
7. Sub-processor engagement rules
8. Data deletion/return upon contract termination
9. Audit rights

---

## 8. Regulatory References

1. **Ley 19.628** — Protección de la Vida Privada (28 agosto 1999)
   - Biblioteca del Congreso Nacional: https://www.bcn.cl/leychile/navegar?idNorma=141599

2. **Ley 21.719** — Ley que Regula la Protección y el Tratamiento de los Datos Personales
   - Biblioteca del Congreso Nacional: https://www.bcn.cl/leychile/navegar?idNorma=1202702

3. **Constitución Política, Art. 19 N°4** — Right to privacy and honor
   - The constitutional guarantee of the right to respect and protection of private life

4. **Código del Trabajo, Art. 154 bis** — Employer obligations regarding worker privacy
   - Restrictions on employer monitoring in the workplace

5. **Reglamento de la Ley 21.719** — Implementing regulations (when enacted)
   - Detailed rules for compliance with the data protection law

6. **Directive (EU) 2016/680** — GDPR reference (comparative)
   - Used as best-practice reference for implementation standards

7. **ILO Code of Practice** — Protection of Workers' Personal Data (1997)
   - International labor standards for employee monitoring

---

## 9. Compliance Checklist

| # | Requirement | Status | Notes |
|---|------------|--------|-------|
| 1 | Lawful basis identified for all processing | ✅ | Consent + legitimate interest |
| 2 | Privacy notice provided to workers | ⬜ | To be implemented in client apps |
| 3 | Consent mechanism (granular, revocable) | ✅ | Per-category consent flags |
| 4 | Data subject rights endpoints | ✅ | Access, export, deletion |
| 5 | Data Protection Impact Assessment | ✅ | Documented in Section 5 |
| 6 | Processing activities register | ✅ | Documented in Section 4 |
| 7 | Data retention policy | ✅ | Documented in Section 6 |
| 8 | Breach notification procedures | ✅ | Documented in Section 1.2.4 |
| 9 | Technical security measures | ✅ | E2E encryption, auth, audit |
| 10 | Data Processing Agreement template | ⬜ | To be drafted |
| 11 | Staff training on data protection | ⬜ | To be scheduled |
| 12 | Supervisory authority registration | ⬜ | When agency is operational |
