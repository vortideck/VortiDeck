buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.3.1")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.2.10")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

// Tauri 2.12's Android library requires every module that depends on it to compile
// against API 36. Some third-party plugins (biometry, keep-screen-on) still declare 34.
// Raising compileSdk only changes the SDK they compile against, not their runtime behavior.
subprojects {
    afterEvaluate {
        extensions.findByType(com.android.build.api.dsl.LibraryExtension::class.java)?.let { android ->
            if ((android.compileSdk ?: 0) < 36) android.compileSdk = 36
        }
    }
}

tasks.register("clean").configure {
    delete("build")
}

