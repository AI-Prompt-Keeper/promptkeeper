// Early Access Signup Form Handler
document.addEventListener('DOMContentLoaded', function() {
    const signupForm = document.getElementById('signupForm');
    const formMessage = document.getElementById('formMessage');
    const emailInput = document.getElementById('email');

    signupForm.addEventListener('submit', async function(e) {
        e.preventDefault();
        
        const email = emailInput.value.trim();
        
        // Basic email validation
        if (!email || !isValidEmail(email)) {
            showMessage('Please enter a valid email address.', 'error');
            return;
        }

        // Disable form during submission
        const submitBtn = signupForm.querySelector('.submit-btn');
        submitBtn.disabled = true;
        submitBtn.textContent = 'Submitting...';

        try {
            // Simulate API call - replace with actual endpoint
            await submitEmail(email);
            
            showMessage('Thanks! We\'ll notify you when we launch.', 'success');
            emailInput.value = '';
            
            // Optional: Track signup (analytics, etc.)
            console.log('Early access signup:', email);
            
        } catch (error) {
            showMessage('Something went wrong. Please try again later.', 'error');
            console.error('Signup error:', error);
        } finally {
            submitBtn.disabled = false;
            submitBtn.textContent = 'Request early access';
        }
    });

    function showMessage(message, type) {
        formMessage.textContent = message;
        formMessage.className = `form-message ${type}`;
        
        // Clear message after 5 seconds
        setTimeout(() => {
            formMessage.textContent = '';
            formMessage.className = 'form-message';
        }, 5000);
    }

    function isValidEmail(email) {
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        return emailRegex.test(email);
    }

    // Simulate API submission - replace with actual API call
    async function submitEmail(email) {
        // Simulate network delay
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        // In production, replace this with actual API call:
        // const response = await fetch('/api/signup', {
        //     method: 'POST',
        //     headers: {
        //         'Content-Type': 'application/json',
        //     },
        //     body: JSON.stringify({ email }),
        // });
        // 
        // if (!response.ok) {
        //     throw new Error('Failed to submit');
        // }
        // 
        // return response.json();
        
        return { success: true };
    }

    // Smooth scroll for in-page anchors
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            const id = this.getAttribute('href');
            if (id === '#') return;
            e.preventDefault();
            const target = document.querySelector(id);
            if (target) {
                target.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        });
    });
});
