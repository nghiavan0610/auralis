/**
 * RevenueCat integration for Auralis subscriptions.
 *
 * Handles subscription purchases, status checks, and tier management
 * across all platforms (macOS, Windows, iOS, Android).
 */

import { Purchases, CustomerInfo, PurchasesError } from '@revenuecat/purchases-js';

// RevenueCat Public SDK Key from environment variable
// See .env.example for configuration
const PUBLIC_SDK_KEY = (import.meta.env.VITE_REVENUECAT_PUBLIC_KEY as string) || '';

// Product IDs configured in RevenueCat dashboard
export const PRODUCTS = {
	FREE: 'auralis_free',
	PRO_MONTHLY: 'auralis_pro_monthly',
} as const;

// Entitlement IDs configured in RevenueCat dashboard
export const ENTITLEMENTS = {
	PRO: 'pro',
} as const;

export type SubscriptionTier = 'free' | 'pro';

export interface SubscriptionStatus {
	tier: SubscriptionTier;
	remaining_summaries: number;
	reset_date: string;
	is_active: boolean;
	will_renew: boolean;
	expire_date?: string;
	is_trial?: boolean;
	is_grace_period?: boolean;
}

export type PurchaseErrorType =
	| 'network_error'
	| 'already_purchased'
	| 'cancelled'
	| 'not_available'
	| 'invalid_configuration'
	| 'unknown';

export interface PurchaseError {
	type: PurchaseErrorType;
	message: string;
	userMessage: string;
	recoverable: boolean;
}

let purchasesInstance: Purchases | null = null;
let isInitialized = false;

// Periodic sync interval (5 minutes)
let syncInterval: ReturnType<typeof setInterval> | null = null;
const SYNC_INTERVAL_MS = 5 * 60 * 1000;

// Storage key for app user ID
const APP_USER_ID_KEY = 'auralis_revenuecat_user_id';

/**
 * Get or create a persistent app user ID for RevenueCat.
 * This ID persists across app restarts for the same user.
 */
function getAppUserID(): string {
	// Check if we already have an ID stored
	let userID = localStorage.getItem(APP_USER_ID_KEY);

	if (!userID) {
		// Generate a new random ID
		userID = 'anon_' + crypto.randomUUID();
		localStorage.setItem(APP_USER_ID_KEY, userID);
	}

	return userID;
}

/**
 * Initialize RevenueCat SDK. Must be called before any other functions.
 */
export async function initRevenueCat(): Promise<boolean> {
	if (isInitialized && purchasesInstance) {
		return true;
	}

	if (!PUBLIC_SDK_KEY) {
		console.error('[RevenueCat] VITE_REVENUECAT_PUBLIC_KEY not set');
		throw createError('invalid_configuration', 'RevenueCat is not configured. Please contact support.');
	}

	try {
		const appUserID = getAppUserID();

		// Pass app user ID explicitly to avoid "undefined" error
		purchasesInstance = new Purchases(PUBLIC_SDK_KEY, appUserID);

		// RevenueCat v1.32+ doesn't require configure() for web platform
		// The Purchases constructor automatically handles initialization

		isInitialized = true;
		return true;
	} catch (error) {
		console.error('[RevenueCat] Initialization failed:', error);
		console.error('[RevenueCat] Error details:', JSON.stringify(error));
		isInitialized = false;
		purchasesInstance = null;

		// Check if it's a network error
		if (error instanceof Error && (error.message.includes('fetch') || error.message.includes('network'))) {
			throw createError('network_error', 'Could not connect to subscription service. Please check your internet connection.');
		}

		throw createError('invalid_configuration', 'Failed to initialize subscription service. Please try again later.');
	}
}

/**
 * Create a standardized error object
 */
function createError(type: PurchaseErrorType, message: string, recoverable = true): PurchaseError {
	const userMessages: Record<PurchaseErrorType, string> = {
		network_error: 'Network error. Please check your connection and try again.',
		already_purchased: 'You already have an active subscription.',
		cancelled: 'Purchase was cancelled.',
		not_available: 'Subscription is not available at this time.',
		invalid_configuration: 'App configuration error. Please contact support.',
		unknown: 'An unexpected error occurred. Please try again.',
	};

	return {
		type,
		message,
		userMessage: userMessages[type] || userMessages.unknown,
		recoverable: type !== 'invalid_configuration' && type !== 'already_purchased',
	};
}

/**
 * Parse PurchasesError and convert to our PurchaseError format
 */
function parsePurchasesError(error: unknown): PurchaseError {
	if (error instanceof PurchasesError) {
		const errorCode = error.code;
		const errorMessage = error.message;

		// Handle specific error codes
		switch (errorCode) {
			case PurchasesError.NETWORK_ERROR:
				return createError('network_error', 'Network error: ' + errorMessage);
			case PurchasesError.INVALID_CREDENTIALS_ERROR:
				return createError('invalid_configuration', 'Invalid API configuration. Please contact support.', false);
			case PurchasesError.OFFLINE_ERROR:
				return createError('network_error', 'You appear to be offline. Please check your connection.');
			case PurchasesError.PRODUCT_ALREADY_PURCHASED_ERROR:
				return createError('already_purchased', 'You already have this subscription.', false);
			case PurchasesError.PURCHASE_CANCELLED_ERROR:
				return createError('cancelled', 'Purchase was cancelled.', false);
			case PurchasesError.PURCHASE_INVALID_ERROR:
				return createError('not_available', 'Purchase was invalid. Please contact support.');
			case PurchasesError.INELIGIBLE_FOR_INTRO_OFFER_ERROR:
				return createError('already_purchased', 'You\'ve already used a trial offer.', false);
			default:
				return createError('unknown', errorMessage);
		}
	}

	if (error instanceof Error) {
		// Check for network-related error messages
		if (error.message.includes('fetch') || error.message.includes('network') || error.message.includes('ECONNREFUSED')) {
			return createError('network_error', error.message);
		}
		return createError('unknown', error.message);
	}

	return createError('unknown', 'An unexpected error occurred');
}

/**
 * Get current subscription status from backend and RevenueCat.
 * Combines RevenueCat customer info with local usage data.
 *
 * This function will:
 * 1. Fetch latest status from RevenueCat (if available)
 * 2. Sync with backend if tiers don't match
 * 3. Return combined status
 */
export async function getSubscriptionStatus(): Promise<SubscriptionStatus> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');

		// Get RevenueCat customer info (if initialized)
		// This fetches the latest from RevenueCat servers, ensuring cross-device sync
		let customerInfo: CustomerInfo | null = null;
		let revenueCatTier: SubscriptionTier | null = null;

		if (purchasesInstance && isInitialized) {
			try {
				// Force refresh from RevenueCat servers (not just cached)
				// This ensures we get the latest subscription status across all devices
				customerInfo = await purchasesInstance.getCustomerInfo();

				// Check active entitlements
				const activeEntitlements = customerInfo.entitlements?.active || {};
				const isPro = ENTITLEMENTS.PRO in activeEntitlements;
				revenueCatTier = isPro ? 'pro' : 'free';

			} catch (error) {
				console.warn('[RevenueCat] Could not fetch from server, using cached:', error);
				// Continue with backend-only status
			}
		}

		// Get usage data from backend
		const backendStatus = await invoke<{
			tier: string;
			remaining_summaries: number;
			reset_date: string;
		}>('get_subscription_status');

		// Determine actual tier - RevenueCat is source of truth if available
		const actualTier = revenueCatTier || (backendStatus.tier as SubscriptionTier);

		// If RevenueCat says Pro but backend says Free, sync them
		// This handles cross-device purchases (e.g., bought on iPhone, using on Mac)
		if (revenueCatTier === 'pro' && backendStatus.tier === 'free') {
			try {
				await syncSubscriptionWithBackend('pro');
			} catch (error) {
				console.warn('[RevenueCat] Failed to sync tier with backend:', error);
				// Continue anyway - use RevenueCat tier
			}
		}
		// If RevenueCat says Free but backend says Pro, sync down
		else if (revenueCatTier === 'free' && backendStatus.tier === 'pro') {
			try {
				await syncSubscriptionWithBackend('free');
			} catch (error) {
				console.warn('[RevenueCat] Failed to sync tier with backend:', error);
			}
		}

		// Build subscription status
		const proEntitlement = customerInfo?.entitlements?.active?.[ENTITLEMENTS.PRO];
		const willRenew = proEntitlement?.willRenew ?? false;
		const expireDate = proEntitlement?.expirationDate
			? new Date(proEntitlement.expirationDate).toISOString()
			: undefined;
		const isTrial = proEntitlement?.periodType === 'trial';
		const isGracePeriod = proEntitlement?.gracePeriodExpiresDate != null;

		return {
			tier: actualTier,
			remaining_summaries: backendStatus.remaining_summaries,
			reset_date: backendStatus.reset_date,
			is_active: customerInfo ? Object.keys(customerInfo.entitlements?.active || {}).length > 0 : false,
			will_renew: willRenew,
			expire_date: expireDate,
			is_trial: isTrial,
			is_grace_period: isGracePeriod,
		};
	} catch (error) {
		console.error('[RevenueCat] Failed to get subscription status:', error);
		throw error;
	}
}

/**
 * Check if user already has an active Pro subscription
 */
export async function hasActiveProSubscription(): Promise<boolean> {
	if (!purchasesInstance || !isInitialized) {
		return false;
	}

	try {
		const customerInfo = await purchasesInstance.getCustomerInfo();
		return ENTITLEMENTS.PRO in (customerInfo.entitlements?.active || {});
	} catch (error) {
		console.warn('[RevenueCat] Could not check subscription status:', error);
		return false;
	}
}

/**
 * Launch the purchase flow for Pro subscription.
 * Returns the new subscription status if successful.
 */
export async function purchasePro(): Promise<SubscriptionStatus> {
	// Initialize if needed
	if (!isInitialized) {
		await initRevenueCat();
	}

	// Check if already subscribed
	const alreadyPro = await hasActiveProSubscription();
	if (alreadyPro) {
		throw createError('already_purchased', 'You already have an active Pro subscription.', false);
	}

	if (!purchasesInstance) {
		throw createError('invalid_configuration', 'Subscription service not available. Please restart the app.', false);
	}

	try {
		// Get available products/offerings
		const offerings = await purchasesInstance.getOfferings();

		// Debug: Log all offerings to see what we got

		const defaultOffering = offerings.current;

		if (!defaultOffering) {
			console.error('[RevenueCat] No current offering found');
			throw createError('not_available', 'Subscription plans are not available at this time. Please try again later.');
		}

		// Debug: Log products in the offering
		// Web SDK uses availablePackages, each package has webBillingProduct
		const availablePackages = defaultOffering.availablePackages || [];

		// Get the monthly package (RevenueCat web SDK uses packages, not products directly)
		const monthlyPackage = availablePackages.find(
			(pkg) => pkg.webBillingProduct?.identifier === PRODUCTS.PRO_MONTHLY
		);

		if (!monthlyPackage) {
			throw createError('not_available', 'Pro subscription is not available. Please contact support.');
		}


		// Launch purchase flow using the package (this opens browser on desktop)
		const { customerInfo: initialCustomerInfo } = await purchasesInstance.purchasePackage(monthlyPackage);


		// After purchase, fetch fresh customer info from RevenueCat servers
		// There might be a delay between payment completion and entitlement activation
		// So we retry a few times to give RevenueCat time to process the purchase
		let customerInfo = initialCustomerInfo;
		let hasPro = false;

		for (let attempt = 0; attempt < 5; attempt++) {
			// Wait a bit before retry (exponential backoff)
			if (attempt > 0) {
				const delay = Math.min(1000 * Math.pow(2, attempt), 5000);
				await new Promise(resolve => setTimeout(resolve, delay));
			}

			// Fetch fresh customer info
			customerInfo = await purchasesInstance.getCustomerInfo();
			hasPro = ENTITLEMENTS.PRO in (customerInfo.entitlements?.active || {});


			if (hasPro) {
				break;
			}
		}

		// Check if purchase was successful
		if (hasPro) {

			// Sync with backend
			await syncSubscriptionWithBackend('pro');

			// Return updated status
			return await getSubscriptionStatus();
		} else {
			// User might have cancelled or there's an issue
			console.error('[RevenueCat] Purchase completed but no Pro entitlement found after retries');
			throw createError('cancelled', 'Purchase was not completed. Please contact support if you completed payment.', false);
		}
	} catch (error) {
		console.error('[RevenueCat] Purchase failed:', error);
		throw parsePurchasesError(error);
	}
}

/**
 * Sync subscription tier with backend after RevenueCat purchase.
 * This ensures our local settings reflect the RevenueCat state.
 */
async function syncSubscriptionWithBackend(tier: SubscriptionTier): Promise<void> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');

		// Fetch current settings first to preserve all fields
		const currentSettings = await invoke<{
			mode: string;
			soniox_api_key: string;
			source_language: string;
			target_language: string;
			translation_type: string;
			audio_source: string;
			opacity: number;
			font_size: number;
			max_lines: number;
			endpoint_delay: number;
			tts_enabled: boolean;
			tts_voice: string;
			tts_rate: number;
			tts_provider: string;
			google_api_key: string;
			elevenlabs_api_key: string;
			summaries_this_month: number;
			last_summary_reset: string;
		}>('get_settings');

		// Update only the subscription_tier field
		await invoke('save_settings', {
			settings: {
				...currentSettings,
				subscription_tier: tier,
			},
		});

	} catch (error) {
		console.error('[RevenueCat] Failed to sync with backend:', error);
		throw createError('unknown', 'Subscription was purchased but could not be synced. Please restart the app.');
	}
}

/**
 * Restore purchases (useful for cross-platform or re-install scenarios).
 * Also useful if subscription was purchased on another device.
 */
export async function restorePurchases(): Promise<SubscriptionStatus> {
	// Initialize if needed
	if (!isInitialized) {
		await initRevenueCat();
	}

	if (!purchasesInstance) {
		throw createError('invalid_configuration', 'Subscription service not available.', false);
	}

	try {

		// This will sync with RevenueCat servers
		const customerInfo = await purchasesInstance.getCustomerInfo();

		// Check if user has active Pro entitlement
		if (customerInfo.entitlements?.active?.[ENTITLEMENTS.PRO]) {
			await syncSubscriptionWithBackend('pro');
		} else {
			// Sync free tier just in case
			await syncSubscriptionWithBackend('free');
		}

		return await getSubscriptionStatus();
	} catch (error) {
		console.error('[RevenueCat] Restore failed:', error);
		throw parsePurchasesError(error);
	}
}

/**
 * Get localized product info for pricing display.
 * Returns null if products are not configured or available.
 */
export async function getProProductInfo(): Promise<{
	productId: string;
	price: string;
	title: string;
	description: string;
} | null> {
	// Initialize if needed (but don't throw if it fails)
	if (!isInitialized) {
		try {
			await initRevenueCat();
		} catch (error) {
			console.warn('[RevenueCat] Could not initialize for product info:', error);
			return null;
		}
	}

	if (!purchasesInstance) {
		return null;
	}

	try {
		const offerings = await purchasesInstance.getOfferings();

		// Debug: Log all offerings to see what we got

		const defaultOffering = offerings.current;

		if (!defaultOffering) {
			console.warn('[RevenueCat] No default offering found');
			return null;
		}

		// Debug: Log products in the offering
		// Web SDK uses availablePackages, each package has webBillingProduct
		const availablePackages = defaultOffering.availablePackages || [];

		// Find the package containing our product
		const pkg = availablePackages.find(
			(p) => p.webBillingProduct?.identifier === PRODUCTS.PRO_MONTHLY
		);

		if (!pkg || !pkg.webBillingProduct) {
			console.warn('[RevenueCat] Pro monthly package not found in offering');
			return null;
		}

		const product = pkg.webBillingProduct;

		return {
			productId: product.identifier,
			price: product.price?.formattedPrice || product.currentPrice?.formattedPrice || '$9.99',
			title: product.title || product.displayName || 'Auralis Pro',
			description: product.description || 'Unlock unlimited summaries and more',
		};
	} catch (error) {
		console.error('[RevenueCat] Failed to get product info:', error);
		return null;
	}
}

/**
 * Check if subscription is in grace period (payment failed but subscription still active)
 */
export async function isInGracePeriod(): Promise<boolean> {
	if (!purchasesInstance || !isInitialized) {
		return false;
	}

	try {
		const customerInfo = await purchasesInstance.getCustomerInfo();
		const proEntitlement = customerInfo.entitlements?.active?.[ENTITLEMENTS.PRO];
		return proEntitlement?.gracePeriodExpiresDate != null;
	} catch {
		return false;
	}
}

/**
 * Get subscription management URL for the current platform
 * Users can use this to manage/cancel their subscription
 */
export function getManageSubscriptionUrl(): string {
	// RevenueCat provides a customer portal URL
	// For now, return a generic URL - in production, configure RevenueCat's customer portal
	return 'https://app.revenuecat.com/customer-portal';
}

/**
 * Reset initialization state (useful for testing or re-initialization)
 */
export function resetInitialization(): void {
	isInitialized = false;
	purchasesInstance = null;
}

/**
 * Start periodic sync with RevenueCat.
 * This ensures subscription status stays up-to-date across devices.
 *
 * Call this when app starts to enable background sync.
 */
export async function startPeriodicSync(): Promise<void> {
	// Clear any existing interval
	if (syncInterval) {
		clearInterval(syncInterval);
	}

	// Do an immediate sync first
	try {
		await getSubscriptionStatus();
	} catch (error) {
		console.warn('[RevenueCat] Initial sync failed:', error);
	}

	// Then start periodic sync
	syncInterval = setInterval(async () => {
		try {
			await getSubscriptionStatus();
		} catch (error) {
			console.warn('[RevenueCat] Periodic sync failed:', error);
		}
	}, SYNC_INTERVAL_MS);

}

/**
 * Stop periodic sync.
 *
 * Call this when app is closing or user logs out.
 */
export function stopPeriodicSync(): void {
	if (syncInterval) {
		clearInterval(syncInterval);
		syncInterval = null;
	}
}

/**
 * Force an immediate sync with RevenueCat.
 * Use this when you need to refresh subscription status right away.
 */
export async function forceSync(): Promise<SubscriptionStatus> {
	return await getSubscriptionStatus();
}
