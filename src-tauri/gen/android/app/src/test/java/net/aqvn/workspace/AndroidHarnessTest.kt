package net.aqvn.workspace

import android.webkit.JavascriptInterface
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AndroidHarnessTest {

    @Test
    fun testPackageAndBuildConfig() {
        assertEquals("net.aqvn.workspace", BuildConfig.APPLICATION_ID)
        assertNotNull(BuildConfig.BUILD_TYPE)
    }

    @Test
    fun testIpcInterfaceContract() {
        val postMessageMethod = Ipc::class.java.getMethod("postMessage", String::class.java)
        assertNotNull("postMessage method must exist", postMessageMethod)
        assertTrue(
            "postMessage must be annotated with @JavascriptInterface",
            postMessageMethod.isAnnotationPresent(JavascriptInterface::class.java)
        )
    }

    @Test
    fun testPermissionHelperUtilityContract() {
        // Verify PermissionHelper methods exist with expected signatures
        val hasPermissionsMethod = PermissionHelper::class.java.getMethod(
            "hasPermissions",
            android.content.Context::class.java,
            Array<String>::class.java
        )
        assertNotNull("hasPermissions method must exist", hasPermissionsMethod)

        val hasDefinedPermissionMethod = PermissionHelper::class.java.getMethod(
            "hasDefinedPermission",
            android.content.Context::class.java,
            String::class.java
        )
        assertNotNull("hasDefinedPermission method must exist", hasDefinedPermissionMethod)

        val getUndefinedPermissionsMethod = PermissionHelper::class.java.getMethod(
            "getUndefinedPermissions",
            android.content.Context::class.java,
            Array<String?>::class.java
        )
        assertNotNull("getUndefinedPermissions method must exist", getUndefinedPermissionsMethod)

        // Empty permission list returns true without dereferencing Context
        assertTrue(PermissionHelper.hasPermissions(null, emptyArray()))
    }
}
