plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "it.notchlite.companion"
    compileSdk = 35
    defaultConfig { applicationId = "it.notchlite.companion"; minSdk = 26; targetSdk = 35; versionCode = 1; versionName = "0.1-preview" }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
}
dependencies {
    implementation("androidx.activity:activity-ktx:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")
    implementation("com.google.zxing:core:3.5.3")
    testImplementation("junit:junit:4.13.2")
}
