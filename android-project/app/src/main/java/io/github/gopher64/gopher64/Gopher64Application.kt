package io.github.gopher64.gopher64

import android.app.Application
import android.content.Intent
import io.github.gopher64.gopher64.services.LoggerService

class Gopher64Application : Application() {

    override fun onCreate() {
        super.onCreate()
        startService(Intent(this, LoggerService::class.java))
    }
}