#!/usr/bin/env bash
#
# End-to-end test for the EC2 scheduler.
#
# Provisions a t3.micro instance via Terraform, runs the scheduler to stop it,
# verifies the state, then starts it again and verifies.
# Cleans up all resources on exit (success or failure).
#
# Usage:
#   bash tests/e2e/run_e2e.sh
#
# Override region:
#   TF_VAR_region=us-east-1 bash tests/e2e/run_e2e.sh
#
set -euo pipefail

# ── Paths ────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/terraform"
IMAGE_NAME="scheduler:e2e-test"

# ── Preflight checks ────────────────────────────────────────────────────────
check_dependency() {
  if ! command -v "$1" &>/dev/null; then
    echo "ERROR: Required tool not found: $1"
    exit 1
  fi
}

check_dependency terraform
check_dependency aws
check_dependency docker

echo "Verifying AWS credentials..."
AWS_ACCOUNT=$(aws sts get-caller-identity --output text --query "Account")
echo "AWS credentials OK (account: ${AWS_ACCOUNT})"

# ── Cleanup trap ─────────────────────────────────────────────────────────────
cleanup() {
  local exit_code=$?
  echo ""
  echo "--- Cleanup: terraform destroy ---"
  terraform -chdir="${TERRAFORM_DIR}" destroy -auto-approve 2>/dev/null || true
  exit ${exit_code}
}
trap cleanup EXIT INT TERM

# ── Helpers ──────────────────────────────────────────────────────────────────
wait_for_state() {
  local instance_id="$1"
  local desired_state="$2"
  local region="$3"
  local max_wait=300
  local interval=15
  local elapsed=0
  local current_state=""

  echo "Waiting for ${instance_id} to reach state: ${desired_state}"

  while [[ ${elapsed} -lt ${max_wait} ]]; do
    current_state=$(aws ec2 describe-instances \
      --region "${region}" \
      --instance-ids "${instance_id}" \
      --query "Reservations[0].Instances[0].State.Name" \
      --output text 2>/dev/null || echo "unknown")

    echo "  [${elapsed}s] state=${current_state}"

    if [[ "${current_state}" == "${desired_state}" ]]; then
      echo "Instance reached state: ${desired_state}"
      return 0
    fi

    sleep "${interval}"
    elapsed=$((elapsed + interval))
  done

  echo "FAIL: timed out after ${max_wait}s waiting for state '${desired_state}' (last: ${current_state})"
  return 1
}

run_scheduler() {
  local action="$1"

  echo "Running scheduler with SCHEDULE_ACTION=${action}"

  local cred_args=()
  if [[ -z "${AWS_ACCESS_KEY_ID:-}" ]]; then
    # Resolve credentials from AWS CLI (handles profiles, SSO, instance roles)
    # and export them as environment variables for Docker.
    eval "$(aws configure export-credentials --format env)"
  fi
  cred_args+=(
    -e "AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}"
    -e "AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}"
    -e "AWS_SESSION_TOKEN=${AWS_SESSION_TOKEN:-}"
  )

  docker run --rm \
    "${cred_args[@]}" \
    -e "SCHEDULE_ACTION=${action}" \
    -e "AWS_REGIONS=${REGION}" \
    -e "TAG_KEY=${TAG_KEY}" \
    -e "TAG_VALUE=${TAG_VALUE}" \
    -e "EC2_SCHEDULE=true" \
    -e "LOG_LEVEL=info" \
    "${IMAGE_NAME}"
}

# ── Build Docker image ───────────────────────────────────────────────────────
echo ""
echo "=== Building Docker image ==="
docker build -t "${IMAGE_NAME}" "${REPO_ROOT}"

# ── Provision test infrastructure ────────────────────────────────────────────
echo ""
echo "=== Provisioning test infrastructure ==="
terraform -chdir="${TERRAFORM_DIR}" init -input=false
terraform -chdir="${TERRAFORM_DIR}" apply -auto-approve -input=false

INSTANCE_ID=$(terraform -chdir="${TERRAFORM_DIR}" output -raw instance_id)
REGION=$(terraform -chdir="${TERRAFORM_DIR}" output -raw region)
TAG_KEY=$(terraform -chdir="${TERRAFORM_DIR}" output -raw tag_key)
TAG_VALUE=$(terraform -chdir="${TERRAFORM_DIR}" output -raw tag_value)

echo ""
echo "Instance ID : ${INSTANCE_ID}"
echo "Region      : ${REGION}"
echo "Tag         : ${TAG_KEY}=${TAG_VALUE}"

# ── Wait for instance to be ready ────────────────────────────────────────────
echo ""
echo "=== Waiting for instance to be running ==="
wait_for_state "${INSTANCE_ID}" "running" "${REGION}"

# ── Test: STOP ───────────────────────────────────────────────────────────────
echo ""
echo "=== Test: SCHEDULE_ACTION=stop ==="
run_scheduler "stop"

wait_for_state "${INSTANCE_ID}" "stopped" "${REGION}"
echo "PASS: instance stopped successfully"

# ── Test: START ──────────────────────────────────────────────────────────────
echo ""
echo "=== Test: SCHEDULE_ACTION=start ==="
run_scheduler "start"

wait_for_state "${INSTANCE_ID}" "running" "${REGION}"
echo "PASS: instance started successfully"

# ── Done (cleanup runs via trap) ─────────────────────────────────────────────
echo ""
echo "=========================================="
echo "  ALL E2E TESTS PASSED"
echo "=========================================="
