def branchName = "${env.BRANCH_NAME}"

node('jenkins-slave-ec2') {
    env.PATH = "/usr/local/bin:/usr/sbin:${env.HOME}/.cargo/bin:${env.PATH}"

    stage('Setup') {
        deleteDir()
        checkout scm
    }

    stage('Build') {
        sh "ssh-agent sh -c 'ssh-add && cargo build --all'"
    }

    stage('Test') {
        sh "cargo test --all"
    }

    stage('Jar') {
        sh "cd platforms/hermes-kotlin/ && ./gradlew jar"
    }

    stage("Kotlin Roundtrip tests") {
        sh "cd platforms/hermes-kotlin && ./gradlew test -Pdebug"
    }

    switch (branchName) {
        case "develop":
        case "master":
            stage("Upload jar") {
                sh """
                    cd platforms/hermes-kotlin/
                    ./gradlew uploadArchives -PnexusUsername="$NEXUS_USERNAME" -PnexusPassword="$NEXUS_PASSWORD"
                """
            }
    }
}
