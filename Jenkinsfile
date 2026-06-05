// Jenkinsfile — multibranch pipeline for arcana-cloud-rust
// Adapted from legacy rust-app-pipeline (single-branch job polling SCM).
//
// Key differences from the legacy XML-embedded script:
//   * `checkout scm` (no hardcoded branch=main)        — supports every branch + every PR
//   * `pollSCM` trigger removed                        — Jenkins multibranch + GitHub webhook drive triggers
//   * "Push to Registry" + "Arch Qube Metrics" gated   — only main pushes to registry; PR builds stay local
//   * SonarQube gets pullrequest.* params on PRs       — PR-decoration in Sonar UI
//   * `dir("${env.PROJECTS_DIR}/...")` blocks removed  — multibranch uses workspace root

pipeline {
    agent any

    options {
        timeout(time: 180, unit: 'MINUTES')
        buildDiscarder(logRotator(numToKeepStr: '20', artifactNumToKeepStr: '1'))
        disableConcurrentBuilds()
        timestamps()
    }

    environment {
        APP_NAME  = "rust-app"
        REGISTRY  = "localhost:5000"
        IMAGE_TAG = "${REGISTRY}/arcana/${APP_NAME}"
        VERSION   = "1.0.0"
    }

    stages {
        stage("Checkout") {
            steps {
                checkout scm
                sh 'git log -1 --oneline'
                script {
                    echo "Branch: ${env.BRANCH_NAME ?: 'unknown'}"
                    echo "PR: ${env.CHANGE_ID ?: 'no'} (target: ${env.CHANGE_TARGET ?: 'n/a'})"
                }
            }
        }

        stage("Cleanup Old Images") {
            // NOTE: the global `docker image prune -f` was REMOVED here — on the
            // shared host daemon it deleted images other concurrent builds were
            // mid-use of ("No such image" flakes). Disk hygiene is handled off-build
            // by the 03:00 cron /data/devops/scripts/docker-cleanup.sh. We still
            // rotate only THIS app's own build-N images (scoped, collision-free).
            steps {
                sh '''
                    docker images --format '{{.Repository}}:{{.Tag}}' \
                        | grep "${APP_NAME}.*build-" \
                        | sort -t- -k2 -rn \
                        | tail -n +4 \
                        | xargs -r docker rmi 2>/dev/null || true
                    docker compose -f docker-compose.test.yml down \
                        --remove-orphans 2>/dev/null || true
                '''
            }
        }

        stage("Docker Compose Build") {
            // Build under the UNIQUE build-N name so buildkit's containerd image
            // store never re-exports a static shared tag. Exporting `:1.0.0` directly
            // fails with `image "...:1.0.0": already exists` when a prior build's
            // :1.0.0 lingers (Cleanup rotates only build-N) or when a concurrent
            // PR build exports the same static name on this shared host daemon.
            // `docker tag` then derives :1.0.0 from build-N — a metadata reassign
            // that always overwrites cleanly. Mirrors arcana-cloud-nodejs/springboot.
            steps {
                sh "CI_BUILD_IMAGE=${IMAGE_TAG}:build-${BUILD_NUMBER} docker compose -f docker-compose.ci.yml build"
                sh "docker tag ${IMAGE_TAG}:build-${BUILD_NUMBER} ${IMAGE_TAG}:${VERSION}"
            }
        }

        stage("Unit Tests") {
            // Blocking: a failing cargo test now fails the build. The inner `|| true`
            // was removed from docker-compose.test.yml's test command so the non-zero
            // cargo exit propagates out of `docker compose run`.
            steps {
                sh "docker compose -f docker-compose.test.yml run --rm --build test"
            }
        }

        stage("Coverage (llvm-cov)") {
            steps {
                catchError(buildResult: 'SUCCESS', stageResult: 'UNSTABLE') {
                    sh "mkdir -p coverage"
                    sh "docker compose -f docker-compose.coverage.yml build coverage || true"
                    sh "docker compose -f docker-compose.coverage.yml run --rm coverage || true"
                    sh '''
                        if [ -f coverage/lcov.info ]; then
                            sed -i "s|SF:/app/|SF:$(pwd)/|g" coverage/lcov.info
                            echo "Fixed LCOV paths: $(head -2 coverage/lcov.info)"
                            echo "Total SF entries: $(grep -c '^SF:' coverage/lcov.info)"
                        else
                            echo "WARNING: coverage/lcov.info not found"
                        fi
                    '''
                }
            }
        }

        stage("Integration: Layered gRPC") {
            // Blocking: a failing layered-gRPC smoke test now fails the build. The
            // teardown / network connect-disconnect lines keep their `|| true` (cleanup
            // must not fail); the gate is integration-smoke-test.sh's own exit code.
            steps {
                sh '''
                    JENKINS_ID=$(hostname)
                    RUST_IMAGE=placeholder docker compose -p arcana-ci-rust-grpc \
                        -f deployment/layered/docker-compose-ci-grpc.yml \
                        down -v --remove-orphans 2>/dev/null || true
                    RUST_IMAGE=${IMAGE_TAG}:build-${BUILD_NUMBER} \
                    docker compose -p arcana-ci-rust-grpc \
                        -f deployment/layered/docker-compose-ci-grpc.yml up -d
                    docker network connect arcana-ci-rust-net $JENKINS_ID 2>/dev/null || true
                    bash scripts/integration-smoke-test.sh \
                        http://arcana-ci-rust-controller:8080 grpc-layered 240
                    docker network disconnect arcana-ci-rust-net $JENKINS_ID 2>/dev/null || true
                '''
            }
            post {
                always {
                    sh '''
                        docker network disconnect arcana-ci-rust-net $(hostname) 2>/dev/null || true
                        RUST_IMAGE=placeholder docker compose -p arcana-ci-rust-grpc \
                            -f deployment/layered/docker-compose-ci-grpc.yml \
                            down -v --remove-orphans 2>/dev/null || true
                    '''
                }
            }
        }

        stage("Integration: K8s gRPC") {
            // Blocking: a failing kind/k8s gRPC smoke test now fails the build.
            steps {
                sh '''#!/bin/bash
                    export PATH="/var/jenkins_home/bin:${PATH}"
                    kind version || { echo "kind not found"; exit 1; }
                    bash scripts/kind-smoke-test.sh "${IMAGE_TAG}:build-${BUILD_NUMBER}" grpc 480
                '''
            }
            post {
                always {
                    sh '''#!/bin/bash
                        export PATH="/var/jenkins_home/bin:${PATH}"
                        kind get clusters 2>/dev/null | grep arcana-ci | while read cl; do
                          kind delete cluster --name "$cl" 2>/dev/null || true
                        done
                    '''
                }
            }
        }

        stage("SonarQube Analysis") {
            // Blocking quality gate. NO sonar.pullrequest.* params: this is SonarQube
            // Community Build, which rejects them ("Developer Edition or above is
            // required") and fails the scan. Analyze every branch/PR with the plain
            // project key. waitForQualityGate() needs a server->Jenkins webhook (not
            // configured here), so poll the compute-engine task named in
            // .scannerwork/report-task.txt, then read the gate status; exit 1 if not OK.
            // The jenkins agent has only curl (no jq), so parse JSON with grep.
            steps {
                withSonarQubeEnv('SonarQube') {
                    sh """sonar-scanner \
                      -Dsonar.projectKey=rust-app \
                      -Dsonar.projectName="Rust App" \
                      -Dsonar.sources=crates \
                      -Dsonar.exclusions=target/**,**/target/**,**/*.proto \
                      -Dsonar.rust.clippy.enabled=false \
                      -Dsonar.scm.disabled=true \
                      -Dsonar.rust.lcov.reportPaths=coverage/lcov.info"""
                    sh '''
                        set -e
                        TOKEN="${SONAR_AUTH_TOKEN:-$SONAR_TOKEN}"
                        RT=.scannerwork/report-task.txt
                        [ -f "$RT" ] || { echo "report-task.txt not found — scanner did not run"; exit 1; }
                        CE_TASK_ID=$(grep '^ceTaskId=' "$RT" | cut -d= -f2-)
                        echo "CE task id: $CE_TASK_ID"
                        ANALYSIS_ID=""
                        for i in $(seq 1 60); do
                            RESP=$(curl -s -u "$TOKEN:" "$SONAR_HOST_URL/api/ce/task?id=$CE_TASK_ID")
                            ST=$(echo "$RESP" | grep -o '"status":"[A-Z_]*"' | head -1 | cut -d'"' -f4)
                            echo "  CE status: ${ST:-?} (try $i)"
                            if [ "$ST" = "SUCCESS" ]; then
                                ANALYSIS_ID=$(echo "$RESP" | grep -o '"analysisId":"[^"]*"' | head -1 | cut -d'"' -f4)
                                break
                            elif [ "$ST" = "FAILED" ] || [ "$ST" = "CANCELED" ]; then
                                echo "CE task ended $ST"; exit 1
                            fi
                            sleep 5
                        done
                        [ -n "$ANALYSIS_ID" ] || { echo "CE task did not finish in time"; exit 1; }
                        GATE=$(curl -s -u "$TOKEN:" "$SONAR_HOST_URL/api/qualitygates/project_status?analysisId=$ANALYSIS_ID")
                        GST=$(echo "$GATE" | grep -o '"status":"[A-Z]*"' | head -1 | cut -d'"' -f4)
                        echo "Quality gate: ${GST:-UNKNOWN}"
                        if [ "$GST" != "OK" ]; then
                            echo "--- gate response ---"; echo "$GATE"
                            exit 1
                        fi
                    '''
                }
            }
        }

        stage("Architecture Qube") {
            // Blocking: arch-qube exits non-zero if the architecture score is below
            // --threshold 90. DinD-safe: this Jenkins talks to the HOST daemon, so a
            // `-v $(pwd):/project` bind mount resolves to a stray host path and scans an
            // empty tree (which is why the old `|| true` gate was fake). Instead copy the
            // source IN via a tar stream and the report OUT with docker cp, both through
            // anonymous volumes (/src, /output) that exist for the container.
            steps {
                sh '''
                    # Per-build container name so concurrent main/PR builds don't
                    # collide on a single static "arcana-arch-qube" name.
                    AQ="arcana-arch-qube-${BUILD_NUMBER}"
                    docker rm -f "$AQ" 2>/dev/null || true
                    docker create --name "$AQ" --network devops_default \
                        -v /src -v /output \
                        arcana.boo/arcana/arch-qube:latest \
                        scan /src --framework rust --no-ai --ci \
                        --format json,markdown -o /output --threshold 90 || exit 1
                    tar --exclude=./.git --exclude=./target --exclude=./.scannerwork \
                        --exclude=./coverage --exclude=./arch-qube-reports \
                        -C . -cf - . \
                        | docker cp - "$AQ":/src || exit 1
                    docker start -a "$AQ"
                    AQ_RC=$?
                    mkdir -p arch-qube-reports
                    docker cp "$AQ":/output/. arch-qube-reports/ 2>/dev/null || true
                    docker rm -f "$AQ" 2>/dev/null || true
                    exit $AQ_RC
                '''
            }
        }

        stage("Image Info") {
            steps {
                sh "docker images --format 'table {{.Repository}}:{{.Tag}}\\t{{.Size}}' | grep ${APP_NAME} || true"
            }
        }

        stage("Push to Registry") {
            when { branch 'main' }
            steps {
                sh "docker push ${IMAGE_TAG}:${VERSION}"
                sh "docker push ${IMAGE_TAG}:build-${BUILD_NUMBER}"
            }
        }

        stage("Arch Qube Metrics") {
            when { branch 'main' }
            steps {
                catchError(buildResult: 'SUCCESS', stageResult: 'SUCCESS') {
                    sh "bash /data/projects/_scripts/arch-qube-metrics.sh \$(pwd) arcana-cloud-rust || true"
                }
            }
        }
    }

    post {
        success { echo "Pipeline SUCCESS - ${APP_NAME}:${VERSION} branch=${env.BRANCH_NAME ?: '?'} pr=${env.CHANGE_ID ?: 'no'}" }
        failure { echo "Pipeline FAILED - branch=${env.BRANCH_NAME ?: '?'} pr=${env.CHANGE_ID ?: 'no'}" }
        always  { echo "Build number ${BUILD_NUMBER} done" }
    }
}
