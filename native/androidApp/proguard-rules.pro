# Rhino's standard library uses named reflective dispatch. Interpretation only.
-keep class org.mozilla.javascript.** { *; }
-dontwarn javax.lang.model.**
# Safe-standard-object scope + optimizationLevel=-1 never loads desktop Java
# bean conversion or Rhino's JVM invokedynamic optimizer on Android.
-dontwarn java.beans.**
-dontwarn jdk.dynalink.**
