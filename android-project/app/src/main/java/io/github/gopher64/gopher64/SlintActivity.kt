package io.github.gopher64.gopher64

import android.app.NativeActivity
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract

class SlintActivity : NativeActivity() {
    companion object {
        init {
            System.loadLibrary("gopher64")
        }

        // N64/64DD ROM + archive extensions the library recognises (mirrors the
        // desktop N64_EXTENSIONS list in gui.rs).
        private val ROM_EXTS = setOf(
            "n64", "v64", "z64", "7z", "zip", "bin", "ndd", "d64"
        )
    }

    private external fun nativeOnActivityResult(requestCode: Int, resultCode: Int, data: Intent?)

    // Delivers the flat list of ROM document-URIs found under a picked folder tree
    // back to Rust (the whole SAF walk stays in Kotlin — far less error-prone than
    // blind JNI Cursor bindings).
    private external fun nativeOnFolderScanned(uris: Array<String>)

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

        // 4 == SELECT_ROM_FOLDER in android.rs.
        if (requestCode == 4 && resultCode == RESULT_OK) {
            val treeUri = data?.data
            if (treeUri != null) {
                contentResolver.takePersistableUriPermission(
                    treeUri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION
                )
                val found = ArrayList<String>()
                scanTree(treeUri, DocumentsContract.getTreeDocumentId(treeUri), found)
                nativeOnFolderScanned(found.toTypedArray())
            } else {
                nativeOnFolderScanned(emptyArray())
            }
            return
        }

        nativeOnActivityResult(requestCode, resultCode, data)
    }

    // Recursively walk the document tree, collecting ROM document-URIs. Symlink
    // cycles aren't possible in SAF trees, and depth is bounded by the store, so a
    // plain recursion is safe. Errors on any subtree are swallowed so one bad
    // folder can't abort the whole scan.
    private fun scanTree(treeUri: Uri, parentDocId: String, out: ArrayList<String>) {
        if (out.size >= 5000) return
        val childrenUri =
            DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
        try {
            contentResolver.query(
                childrenUri,
                arrayOf(
                    DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                    DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                    DocumentsContract.Document.COLUMN_MIME_TYPE
                ),
                null, null, null
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    val docId = cursor.getString(0)
                    val name = cursor.getString(1) ?: ""
                    val mime = cursor.getString(2) ?: ""
                    if (mime == DocumentsContract.Document.MIME_TYPE_DIR) {
                        scanTree(treeUri, docId, out)
                    } else {
                        val ext = name.substringAfterLast('.', "").lowercase()
                        if (ext in ROM_EXTS) {
                            out.add(
                                DocumentsContract
                                    .buildDocumentUriUsingTree(treeUri, docId)
                                    .toString()
                            )
                        }
                    }
                }
            }
        } catch (_: Exception) {
            // Unreadable subtree: skip it, keep whatever was already found.
        }
    }
}
