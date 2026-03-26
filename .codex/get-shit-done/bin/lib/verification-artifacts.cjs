const fs = require('fs');
const path = require('path');
const { comparePhaseNum, getMilestoneInfo, getMilestonePhaseFilter, getRoadmapPhaseInternal, planningPaths, toPosixPath } = require('./core.cjs');
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

function parseRequirementsCoverageRows(content) {
  const sectionMatch = content.match(/##\s*Requirements Coverage\s*\n([\s\S]*?)(?=\n##\s|$)/i);
  if (!sectionMatch) return [];

  return sectionMatch[1]
    .split('\n')
    .map(line => line.trim())
    .filter(line => line.startsWith('|'))
    .filter(line => !/^\|\s*-/.test(line))
    .slice(1)
    .map(line => line.split('|').slice(1, -1).map(cell => cell.trim()))
    .filter(cells => cells.length >= 2 && cells[0] && !/^Requirement$/i.test(cells[0]))
    .map(cells => ({
      requirement: cells[0],
      status: cells[1] || 'unknown',
      blocking_issue: cells[2] || '',
    }));
}

function collectMilestoneVerificationState(cwd, options = {}) {
  const milestone = options.milestone || getMilestoneInfo(cwd);
  const phasesDir = planningPaths(cwd).phases;
  const isDirInMilestone = getMilestonePhaseFilter(cwd);
  const phaseDirs = [];

  try {
    const entries = fs.readdirSync(phasesDir, { withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      if (!isDirInMilestone(entry.name)) continue;
      phaseDirs.push(entry.name);
    }
  } catch {}

  phaseDirs.sort((a, b) => comparePhaseNum(a, b));

  const phases = phaseDirs.map(dir => {
    const phaseMatch = dir.match(/^(\d+[A-Z]?(?:\.\d+)*)/i);
    const phaseNumber = phaseMatch ? phaseMatch[1] : dir;
    const roadmapPhase = getRoadmapPhaseInternal(cwd, phaseNumber);
    const phaseName = roadmapPhase?.phase_name || dir.replace(/^\d+[A-Z]?(?:\.\d+)*-?/, '').replace(/-/g, ' ');
    const phaseDir = path.join(phasesDir, dir);
    const inspection = inspectVerificationArtifacts(phaseDir);
    const debtItems = buildVerificationDebtItems(inspection);
    let requirements = [];

    if (inspection.verification_path && fs.existsSync(inspection.verification_path)) {
      const content = fs.readFileSync(inspection.verification_path, 'utf-8');
      requirements = parseRequirementsCoverageRows(content);
    }

    return {
      phase_number: phaseNumber,
      phase_name: phaseName,
      phase_dir: toPosixPath(path.relative(cwd, phaseDir)),
      verification_file: inspection.verification_file,
      verification_path: inspection.verification_path
        ? toPosixPath(path.relative(cwd, inspection.verification_path))
        : null,
      status: inspection.status,
      score: inspection.score,
      verified_at: inspection.verified_at || inspection.verification_iso,
      debt_items: debtItems,
      requirements,
    };
  });

  const summary = {
    total_phases: phases.length,
    passed: phases.filter(phase => phase.status === 'passed' && phase.debt_items.length === 0).length,
    human_needed: phases.filter(phase => phase.status === 'human_needed').length,
    gaps_found: phases.filter(phase => phase.status === 'gaps_found').length,
    missing: phases.filter(phase => phase.status === 'missing').length,
    debt_items: phases.reduce((sum, phase) => sum + phase.debt_items.length, 0),
  };

  return {
    milestone_version: milestone.version,
    milestone_name: milestone.name,
    generated_at: new Date().toISOString(),
    phases,
    summary,
  };
}

function renderMilestoneVerificationArchive(snapshot, options = {}) {
  const lines = [
    `# Verification Archive: ${snapshot.milestone_version} ${snapshot.milestone_name}`,
    '',
    `**Generated:** ${snapshot.generated_at}`,
    '',
    '## Summary',
    '',
    `- Phases in milestone: ${snapshot.summary.total_phases}`,
    `- Fully passed phases: ${snapshot.summary.passed}`,
    `- Human verification outstanding: ${snapshot.summary.human_needed}`,
    `- Verification gaps found: ${snapshot.summary.gaps_found}`,
    `- Missing verification artifacts: ${snapshot.summary.missing}`,
    `- Verification debt items: ${snapshot.summary.debt_items}`,
    '',
    '## Phase Verification Matrix',
    '',
    '| Phase | Name | Status | Score | Verification Artifact | Debt |',
    '|-------|------|--------|-------|-----------------------|------|',
  ];

  for (const phase of snapshot.phases) {
    const debtSummary = phase.debt_items.length
      ? phase.debt_items.map(item => item.category).join(', ')
      : 'none';
    const phaseDirName = phase.phase_dir.split('/').slice(-1)[0];
    const artifactPath = options.archivePhases && phase.verification_file
      ? `.planning/milestones/${snapshot.milestone_version}-phases/${phaseDirName}/${phase.verification_file}`
      : (phase.verification_path || '(missing)');
    lines.push(`| ${phase.phase_number} | ${phase.phase_name} | ${phase.status} | ${phase.score || '-'} | ${artifactPath} | ${debtSummary} |`);
  }

  lines.push('', '## Requirement Coverage Snapshot', '');
  for (const phase of snapshot.phases) {
    lines.push(`### Phase ${phase.phase_number}: ${phase.phase_name}`);
    if (!phase.requirements.length) {
      lines.push('', '- No requirements coverage table recorded in this phase verification artifact.', '');
      continue;
    }
    lines.push('', '| Requirement | Status | Blocking Issue |', '|-------------|--------|----------------|');
    for (const requirement of phase.requirements) {
      lines.push(`| ${requirement.requirement} | ${requirement.status} | ${requirement.blocking_issue} |`);
    }
    lines.push('');
  }

  lines.push('## Accepted Verification Debt', '');
  const debtLines = snapshot.phases.flatMap(phase => phase.debt_items.map(item => `- Phase ${phase.phase_number}: ${item.category} — ${item.reason}`));
  if (debtLines.length === 0) {
    lines.push('- None');
  } else {
    lines.push(...debtLines);
  }

  lines.push('');
  return lines.join('\n');
}

module.exports = {
  buildVerificationDebtItems,
  collectMilestoneVerificationState,
  inspectVerificationArtifacts,
  renderMilestoneVerificationArchive,
};
