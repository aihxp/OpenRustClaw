const fs = require('fs');
const path = require('path');
const { extractFrontmatter } = require('./frontmatter.cjs');

function listMatchingFiles(phaseDir, predicate) {
  try {
    return fs.readdirSync(phaseDir)
      .filter(predicate)
      .map(name => {
        const fullPath = path.join(phaseDir, name);
        let stat = null;
        try {
          stat = fs.statSync(fullPath);
        } catch {}
        return { name, fullPath, stat };
      })
      .filter(entry => entry.stat && entry.stat.isFile());
  } catch {
    return [];
  }
}

function findLatestFile(entries) {
  if (!entries.length) return null;
  return entries.reduce((latest, entry) => {
    if (!latest) return entry;
    return entry.stat.mtimeMs > latest.stat.mtimeMs ? entry : latest;
  }, null);
}

function inspectVerificationArtifacts(phaseDir) {
  const verificationFiles = listMatchingFiles(
    phaseDir,
    name => (name.endsWith('-VERIFICATION.md') || name === 'VERIFICATION.md')
  );
  const newestVerification = findLatestFile(verificationFiles);
  const summaryFiles = listMatchingFiles(
    phaseDir,
    name => (name.endsWith('-SUMMARY.md') || name === 'SUMMARY.md')
  );
  const uatFiles = listMatchingFiles(
    phaseDir,
    name => name.endsWith('.md') && name.includes('-UAT')
  );
  const evidenceFiles = [...summaryFiles, ...uatFiles];
  const latestEvidence = findLatestFile(evidenceFiles);

  if (!newestVerification) {
    return {
      exists: false,
      status: 'missing',
      verification_file: null,
      verification_path: null,
      verification_mtime: null,
      verification_iso: null,
      verified_at: null,
      score: null,
      stale: false,
      latest_evidence_file: latestEvidence?.name || null,
      latest_evidence_mtime: latestEvidence?.stat.mtimeMs || null,
      blocking_reasons: [
        {
          code: 'missing_verification',
          message: 'No VERIFICATION.md artifact exists for this phase.',
        },
      ],
      warnings: [],
    };
  }

  const content = fs.readFileSync(newestVerification.fullPath, 'utf-8');
  const frontmatter = extractFrontmatter(content);
  const blocking = [];
  const warnings = [];
  const status = String(frontmatter.status || '').trim() || 'pending';
  const verifiedAt = String(frontmatter.verified || '').trim() || null;
  const score = String(frontmatter.score || '').trim() || null;

  const missingFields = ['phase', 'verified', 'status', 'score'].filter(field => !frontmatter[field]);
  if (missingFields.length > 0) {
    blocking.push({
      code: 'invalid_verification',
      message: `${newestVerification.name} is missing required frontmatter: ${missingFields.join(', ')}.`,
    });
  }

  if (status === 'pending') {
    blocking.push({
      code: 'pending_verification',
      message: `${newestVerification.name} is still pending.`,
    });
  } else if (status === 'gaps_found') {
    blocking.push({
      code: 'verification_gaps',
      message: `${newestVerification.name} reports unresolved gaps.`,
    });
  } else if (status === 'human_needed') {
    warnings.push(`${newestVerification.name}: needs human verification`);
  } else if (status !== 'passed') {
    blocking.push({
      code: 'invalid_verification',
      message: `${newestVerification.name} has unsupported status "${status}".`,
    });
  }

  const stale = !!(latestEvidence && newestVerification.stat.mtimeMs < latestEvidence.stat.mtimeMs);
  if (stale) {
    blocking.push({
      code: 'stale_verification',
      message: `${newestVerification.name} predates ${latestEvidence.name}.`,
    });
  }

  return {
    exists: true,
    status,
    verification_file: newestVerification.name,
    verification_path: newestVerification.fullPath,
    verification_mtime: newestVerification.stat.mtimeMs,
    verification_iso: new Date(newestVerification.stat.mtimeMs).toISOString(),
    verified_at: verifiedAt,
    score,
    stale,
    latest_evidence_file: latestEvidence?.name || null,
    latest_evidence_mtime: latestEvidence?.stat.mtimeMs || null,
    blocking_reasons: blocking,
    warnings,
  };
}

function buildVerificationDebtItems(inspection) {
  if (!inspection) return [];

  const items = inspection.blocking_reasons.map(reason => ({
    name: inspection.verification_file || 'VERIFICATION.md',
    result: inspection.status || 'missing',
    category: reason.code,
    reason: reason.message,
  }));

  if (inspection.status === 'human_needed') {
    items.push({
      name: inspection.verification_file,
      result: 'human_needed',
      category: 'human_uat',
      reason: 'Automated verification passed, but human checks still need explicit review.',
    });
  }

  return items;
}

module.exports = {
  buildVerificationDebtItems,
  inspectVerificationArtifacts,
};
