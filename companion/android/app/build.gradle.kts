import java.util.zip.ZipFile
import java.util.zip.ZipInputStream

plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "it.notchlite.companion"
    compileSdk = 35
    defaultConfig { applicationId = "it.notchlite.companion"; minSdk = 26; targetSdk = 35; versionCode = 1; versionName = "0.1-preview" }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
    sourceSets["main"].assets.srcDir(layout.buildDirectory.dir("generated/notices"))
}

val dependencyNotices = tasks.register("dependencyNotices") {
    val runtime = configurations.getByName("debugRuntimeClasspath")
    inputs.files(runtime)
    inputs.file(rootProject.file("../NOTICE"))
    inputs.file(rootProject.file("../../LICENSE"))
    val output = layout.buildDirectory.file("generated/notices/NOTICE.txt")
    outputs.file(output)
    doLast {
        val result = StringBuilder(rootProject.file("../NOTICE").readText())
        result.append("\n\n").append(rootProject.file("../../LICENSE").readText())
        fun legal(name: String) = name.substringAfterLast('/').uppercase().let { it.startsWith("LICENSE") || it.startsWith("NOTICE") || it.startsWith("COPYING") }
        for (artifact in runtime.resolvedConfiguration.resolvedArtifacts.sortedBy { it.moduleVersion.id.toString() }) {
            result.append("\n\nRESOLVED COMPONENT: ").append(artifact.moduleVersion.id.toString()).append('\n')
            ZipFile(artifact.file).use { zip ->
                for (entry in zip.entries()) {
                    if (!entry.isDirectory && legal(entry.name)) result.append(zip.getInputStream(entry).bufferedReader().use { it.readText() }).append('\n')
                    if (entry.name == "classes.jar") ZipInputStream(zip.getInputStream(entry)).use { nested ->
                        while (true) {
                            val child = nested.nextEntry ?: break
                            if (!child.isDirectory && legal(child.name)) result.append(nested.readBytes().toString(Charsets.UTF_8)).append('\n')
                        }
                    }
                }
            }
        }
        output.get().asFile.apply { parentFile.mkdirs(); writeText(result.toString()) }
    }
}
tasks.named("preBuild").configure { dependsOn(dependencyNotices) }
dependencies {
    implementation("androidx.activity:activity-ktx:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")
    implementation("com.google.zxing:core:3.5.3")
    testImplementation("junit:junit:4.13.2")
}
